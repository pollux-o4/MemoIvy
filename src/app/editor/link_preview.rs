//! 링크 위에 잠시 머물면 실제 목적지를 보여 주는 떠 있는 카드.
//! 포커스를 가져가지 않아 입력·IME가 끊기지 않고, 네트워크 요청은 하지 않는다.
use crate::presentation::markdown::{self, Link};
use gtk::{gdk, glib, pango, prelude::*, Box as GtkBox, Button, Label, Orientation, TextView};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

/// 지나가는 포인터에 뜨지 않을 만큼만 기다린다.
const HOVER_DELAY: Duration = Duration::from_millis(350);
/// 키보드는 타이핑 도중 끼어들지 않도록 더 오래 기다린다.
const CARET_DELAY: Duration = Duration::from_millis(600);
/// 카드가 방금 떠 있었으면 옆 링크로 옮겨 갈 때 기다리지 않는다.
const SWITCH_DELAY: Duration = Duration::from_millis(60);
const SWITCH_WINDOW: Duration = Duration::from_millis(400);
/// 링크에서 카드로 포인터를 옮기는 동안 닫히지 않게 한다.
const CLOSE_DELAY: Duration = Duration::from_millis(220);
/// CSS `.closing` 전환 길이와 맞춘다.
const EXIT: Duration = Duration::from_millis(90);
const COPIED: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Source {
    Pointer,
    Caret,
}

struct Card {
    popover: gtk::Popover,
    badge: Label,
    title: Label,
    address: Label,
    warning: Label,
    open: Button,
    copy: Button,
    edit: Button,
}

pub(super) struct LinkPreview {
    view: glib::WeakRef<TextView>,
    card: RefCell<Option<Rc<Card>>>,
    shown: RefCell<Option<(Link, Source)>>,
    pending: RefCell<Option<(Link, Source)>>,
    show_timer: RefCell<Option<glib::SourceId>>,
    hide_timer: RefCell<Option<glib::SourceId>>,
    exit_timer: RefCell<Option<glib::SourceId>>,
    copied_timer: RefCell<Option<glib::SourceId>>,
    closed_at: Cell<Option<Instant>>,
    over_card: Cell<bool>,
    /// Esc로 닫은 링크. 커서가 그 안에서 움직여도 다시 띄우지 않는다.
    dismissed: RefCell<Option<std::ops::Range<i32>>>,
    hover_tag: gtk::TextTag,
}

fn clear(timer: &RefCell<Option<glib::SourceId>>) {
    if let Some(id) = timer.borrow_mut().take() {
        id.remove();
    }
}

/// GTK 표면 안에서만 쓰는 작은 텍스트 버튼. 눌러도 편집기 포커스를 유지한다.
fn action(label: &str, tip: Option<&str>) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("link-action");
    button.set_focusable(false);
    button.set_focus_on_click(false);
    button.set_tooltip_text(tip);
    button
}

pub(super) fn open_url(view: &TextView, url: &str) {
    let window = view.root().and_downcast::<gtk::Window>();
    // shortcut: FMT-001 링크 제스처를 유지, UriLauncher 전환 시 임시 핸들러 검증과 함께 교체.
    #[allow(deprecated)]
    gtk::show_uri(window.as_ref(), url, gdk::CURRENT_TIME);
}

impl LinkPreview {
    pub(super) fn new(view: &TextView, hover_tag: gtk::TextTag) -> Rc<Self> {
        let preview = Rc::new(Self {
            view: view.downgrade(),
            card: RefCell::new(None),
            shown: RefCell::new(None),
            pending: RefCell::new(None),
            show_timer: RefCell::new(None),
            hide_timer: RefCell::new(None),
            exit_timer: RefCell::new(None),
            copied_timer: RefCell::new(None),
            closed_at: Cell::new(None),
            over_card: Cell::new(false),
            dismissed: RefCell::new(None),
            hover_tag,
        });
        // GtkTextView는 파괴될 때 자식을 모두 지우려 돌며 자기 것이 아닌 표면이 있으면 끝나지 않는다.
        // 그래서 보이지 않게 되는 즉시 떼어 내고, 다음에 필요할 때 다시 만든다.
        let weak = Rc::downgrade(&preview);
        view.connect_unmap(move |_| {
            if let Some(preview) = weak.upgrade() {
                preview.teardown();
            }
        });
        preview
    }

    fn teardown(&self) {
        for timer in [
            &self.show_timer,
            &self.hide_timer,
            &self.exit_timer,
            &self.copied_timer,
        ] {
            clear(timer);
        }
        self.pending.replace(None);
        self.shown.replace(None);
        self.over_card.set(false);
        if let Some(view) = self.view.upgrade() {
            self.mark(&view, None);
        }
        if let Some(card) = self.card.borrow_mut().take() {
            card.popover.unparent();
        }
    }

    fn card(self: &Rc<Self>, view: &TextView) -> Rc<Card> {
        if let Some(card) = &*self.card.borrow() {
            return card.clone();
        }
        let popover = gtk::Popover::new();
        crate::app::shared::menu_popover(&popover);
        popover.add_css_class("link-preview");
        // 포인터를 잡지도, 키 입력을 가져가지도 않는다.
        popover.set_autohide(false);
        popover.set_can_focus(false);
        popover.set_position(gtk::PositionType::Bottom);
        // 가운데 정렬은 왼쪽 링크에서 창 밖으로 밀려난다. 링크 시작점에 맞춘다.
        popover.set_halign(gtk::Align::Start);
        let body = GtkBox::new(Orientation::Vertical, 8);
        body.add_css_class("link-body");
        let head = GtkBox::new(Orientation::Horizontal, 10);
        let badge = Label::new(None);
        badge.add_css_class("link-badge");
        badge.set_valign(gtk::Align::Start);
        head.append(&badge);
        let text = GtkBox::new(Orientation::Vertical, 2);
        text.set_hexpand(true);
        text.set_valign(gtk::Align::Center);
        let title = Label::new(None);
        title.add_css_class("link-title");
        title.set_xalign(0.);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.set_max_width_chars(34);
        title.set_width_chars(24);
        text.append(&title);
        let address = Label::new(None);
        address.add_css_class("link-address");
        address.set_xalign(0.);
        address.set_wrap(true);
        address.set_wrap_mode(pango::WrapMode::Char);
        address.set_lines(2);
        address.set_ellipsize(pango::EllipsizeMode::End);
        address.set_max_width_chars(34);
        text.append(&address);
        head.append(&text);
        body.append(&head);
        let warning = Label::new(None);
        warning.add_css_class("link-warning");
        warning.set_xalign(0.);
        warning.set_wrap(true);
        warning.set_max_width_chars(38);
        body.append(&warning);
        let actions = GtkBox::new(Orientation::Horizontal, 4);
        let open = action("열기", Some("기본 앱에서 열기 · Ctrl+클릭"));
        open.add_css_class("primary");
        let copy = action("주소 복사", None);
        let edit = action("주소 수정", None);
        actions.append(&open);
        actions.append(&copy);
        actions.append(&edit);
        body.append(&actions);
        popover.set_child(Some(&body));
        popover.set_parent(view);
        // 카드 위에 있는 동안은 닫지 않는다(WCAG 1.4.13 hoverable).
        let hover = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(self);
        hover.connect_enter(move |_, _, _| {
            if let Some(preview) = weak.upgrade() {
                preview.over_card.set(true);
                clear(&preview.hide_timer);
            }
        });
        let weak = Rc::downgrade(self);
        hover.connect_leave(move |_| {
            if let Some(preview) = weak.upgrade() {
                preview.over_card.set(false);
                preview.schedule_hide();
            }
        });
        popover.add_controller(hover);
        let weak = Rc::downgrade(self);
        open.connect_clicked(move |_| {
            if let Some(preview) = weak.upgrade() {
                let link = preview
                    .shown
                    .borrow()
                    .as_ref()
                    .map(|(link, _)| link.clone());
                if let (Some(link), Some(view)) = (link, preview.view.upgrade()) {
                    open_url(&view, &link.url);
                }
                preview.hide();
            }
        });
        let weak = Rc::downgrade(self);
        copy.connect_clicked(move |button| {
            let Some(preview) = weak.upgrade() else {
                return;
            };
            let link = preview
                .shown
                .borrow()
                .as_ref()
                .map(|(link, _)| link.clone());
            if let (Some(link), Some(view)) = (link, preview.view.upgrade()) {
                view.clipboard().set_text(&link.url);
                button.set_label("복사됨");
                button.add_css_class("confirmed");
                clear(&preview.copied_timer);
                let weak = Rc::downgrade(&preview);
                let id = glib::timeout_add_local_once(COPIED, move || {
                    if let Some(preview) = weak.upgrade() {
                        preview.copied_timer.borrow_mut().take();
                        preview.reset_copy();
                    }
                });
                preview.copied_timer.replace(Some(id));
            }
        });
        let weak = Rc::downgrade(self);
        edit.connect_clicked(move |_| {
            let Some(preview) = weak.upgrade() else {
                return;
            };
            let link = preview
                .shown
                .borrow()
                .as_ref()
                .map(|(link, _)| link.clone());
            preview.hide();
            if let (Some(link), Some(view)) = (link, preview.view.upgrade()) {
                let buffer = view.buffer();
                buffer.select_range(
                    &buffer.iter_at_offset(link.url_range.start),
                    &buffer.iter_at_offset(link.url_range.end),
                );
                view.grab_focus();
            }
        });
        let card = Rc::new(Card {
            popover,
            badge,
            title,
            address,
            warning,
            open,
            copy,
            edit,
        });
        self.card.replace(Some(card.clone()));
        card
    }

    fn reset_copy(&self) {
        if let Some(card) = &*self.card.borrow() {
            card.copy.set_label("주소 복사");
            card.copy.remove_css_class("confirmed");
        }
    }

    fn fill(&self, card: &Card, view: &TextView, link: &Link) {
        let buffer = view.buffer();
        let label = buffer
            .text(
                &buffer.iter_at_offset(link.range.start),
                &buffer.iter_at_offset(link.range.end),
                true,
            )
            .to_string();
        // 이름의 서식 문법이 판별과 표시에 끼어들지 않게 평문으로 읽는다.
        let label = markdown::plain_line(&label);
        let info = markdown::link_info(&link.url, &label);
        // 이름이 주소와 같으면 두 번 보이지 않게 호스트를 제목으로 올린다.
        let named = label != link.url && !label.trim().is_empty();
        card.title
            .set_text(if named { label.trim() } else { &info.host });
        let address = if named {
            format!("{}{}", info.host, info.rest)
        } else {
            info.rest.clone()
        };
        card.address.set_text(&address);
        card.address.set_visible(!address.is_empty());
        // 호스트를 강조해 위장 주소를 알아보기 쉽게 한다.
        let attrs = pango::AttrList::new();
        if named {
            let mut weight = pango::AttrInt::new_weight(pango::Weight::Semibold);
            weight.set_start_index(0);
            weight.set_end_index(info.host.len() as u32);
            attrs.insert(weight);
        }
        card.address.set_attributes(Some(&attrs));
        card.address.set_tooltip_text(Some(&link.url));
        let initial = info
            .host
            .trim_start_matches("www.")
            .chars()
            .next()
            .map_or_else(|| "?".to_owned(), |c| c.to_uppercase().to_string());
        card.badge
            .set_text(if info.mail { "@" } else { initial.as_str() });
        card.warning.set_visible(info.warning.is_some());
        card.warning.set_text(info.warning.unwrap_or_default());
        card.open
            .set_label(if info.mail { "메일 쓰기" } else { "열기" });
        card.edit.set_visible(view.is_editable());
        self.reset_copy();
    }

    /// 카드가 가리킬 자리. 숨긴 문법이 있는 줄은 글자 가로 위치가 믿을 만하지 않아 세로만 글자에서 읽는다.
    pub(super) fn anchor(view: &TextView, offset: i32, x: Option<i32>) -> gdk::Rectangle {
        let spot = view.iter_location(&view.buffer().iter_at_offset(offset));
        let (left, y) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, spot.x(), spot.y());
        gdk::Rectangle::new(x.unwrap_or(left), y, 1, spot.height())
    }

    /// 포인터가 링크 위에 있거나 벗어났다.
    pub(super) fn hover(self: &Rc<Self>, link: Option<&Link>, anchor: gdk::Rectangle) {
        let current = self.shown.borrow().as_ref().map(|(l, _)| l.owner.clone());
        match link {
            Some(link) if current.as_ref() == Some(&link.owner) => {
                clear(&self.hide_timer);
                clear(&self.show_timer);
                self.pending.replace(None);
                if let Some(shown) = &mut *self.shown.borrow_mut() {
                    shown.1 = Source::Pointer;
                }
            }
            Some(link) => {
                clear(&self.hide_timer);
                let same = self
                    .pending
                    .borrow()
                    .as_ref()
                    .is_some_and(|(l, s)| l.owner == link.owner && *s == Source::Pointer);
                if same {
                    return;
                }
                self.schedule_show(link.clone(), Source::Pointer, anchor, HOVER_DELAY);
            }
            None => self.pointer_left(),
        }
    }

    pub(super) fn pointer_left(self: &Rc<Self>) {
        if self
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|(_, source)| *source == Source::Pointer)
        {
            clear(&self.show_timer);
            self.pending.replace(None);
        }
        if self
            .shown
            .borrow()
            .as_ref()
            .is_some_and(|(_, source)| *source == Source::Pointer)
        {
            self.schedule_hide();
        }
    }

    /// 커서가 링크 안에 있거나 벗어났다. 포인터로 띄운 카드는 건드리지 않는다.
    pub(super) fn caret(self: &Rc<Self>, link: Option<&Link>, anchor: gdk::Rectangle) {
        let shown = self
            .shown
            .borrow()
            .as_ref()
            .map(|(l, s)| (l.owner.clone(), *s));
        let owner = link.map(|link| link.owner.clone());
        if *self.dismissed.borrow() != owner {
            self.dismissed.replace(None);
        }
        match link {
            Some(link) if self.dismissed.borrow().as_ref() == Some(&link.owner) => {}
            Some(link) => {
                if shown.is_some_and(|(owner, _)| owner == link.owner) {
                    return;
                }
                let same = self
                    .pending
                    .borrow()
                    .as_ref()
                    .is_some_and(|(l, _)| l.owner == link.owner);
                if !same {
                    self.schedule_show(link.clone(), Source::Caret, anchor, CARET_DELAY);
                }
            }
            None => {
                if self
                    .pending
                    .borrow()
                    .as_ref()
                    .is_some_and(|(_, source)| *source == Source::Caret)
                {
                    clear(&self.show_timer);
                    self.pending.replace(None);
                }
                if shown.is_some_and(|(_, source)| source == Source::Caret) {
                    self.hide();
                }
            }
        }
    }

    fn schedule_show(
        self: &Rc<Self>,
        link: Link,
        source: Source,
        anchor: gdk::Rectangle,
        delay: Duration,
    ) {
        clear(&self.show_timer);
        let recent = self.shown.borrow().is_some()
            || self
                .closed_at
                .get()
                .is_some_and(|at| at.elapsed() < SWITCH_WINDOW);
        let delay = if recent && source == Source::Pointer {
            SWITCH_DELAY
        } else {
            delay
        };
        self.pending.replace(Some((link, source)));
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(delay, move || {
            if let Some(preview) = weak.upgrade() {
                preview.show_timer.borrow_mut().take();
                preview.show_pending(anchor);
            }
        });
        self.show_timer.replace(Some(id));
    }

    fn show_pending(self: &Rc<Self>, anchor: gdk::Rectangle) {
        let Some((link, source)) = self.pending.borrow_mut().take() else {
            return;
        };
        let Some(view) = self.view.upgrade() else {
            return;
        };
        if !view.is_mapped() {
            return;
        }
        let card = self.card(&view);
        clear(&self.hide_timer);
        clear(&self.exit_timer);
        card.popover.remove_css_class("closing");
        self.fill(&card, &view, &link);
        self.mark(&view, Some(&link));
        card.popover.set_pointing_to(Some(&anchor));
        let reopening = card.popover.is_visible();
        self.shown.replace(Some((link, source)));
        if !reopening {
            card.popover.popup();
        }
    }

    fn mark(&self, view: &TextView, link: Option<&Link>) {
        let buffer = view.buffer();
        buffer.remove_tag(&self.hover_tag, &buffer.start_iter(), &buffer.end_iter());
        if let Some(link) = link {
            buffer.apply_tag(
                &self.hover_tag,
                &buffer.iter_at_offset(link.range.start),
                &buffer.iter_at_offset(link.range.end),
            );
        }
    }

    fn schedule_hide(self: &Rc<Self>) {
        clear(&self.hide_timer);
        if self.shown.borrow().is_none() || self.over_card.get() {
            return;
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(CLOSE_DELAY, move || {
            if let Some(preview) = weak.upgrade() {
                preview.hide_timer.borrow_mut().take();
                preview.hide();
            }
        });
        self.hide_timer.replace(Some(id));
    }

    /// 다시 띄운 팝오버 표면은 X11에서 포인터 진입을 받지 못해 카드로 옮겨 가도 닫힌다. 매번 새로 만든다.
    fn discard_card(&self) {
        clear(&self.copied_timer);
        if let Some(card) = self.card.borrow_mut().take() {
            card.popover.popdown();
            card.popover.unparent();
        }
    }

    /// 사용자가 직접 닫았다. 같은 링크 안에 있는 동안에는 다시 띄우지 않는다.
    pub(super) fn dismiss(self: &Rc<Self>) {
        let owner = self.shown.borrow().as_ref().map(|(l, _)| l.owner.clone());
        self.dismissed.replace(owner);
        self.hide();
    }

    pub(super) fn is_shown(&self) -> bool {
        self.shown.borrow().is_some()
    }

    /// 대기·표시 중인 카드를 모두 닫는다. 입력·Esc·스크롤처럼 즉시 물러나야 할 때 쓴다.
    pub(super) fn hide(self: &Rc<Self>) {
        clear(&self.show_timer);
        clear(&self.hide_timer);
        self.pending.replace(None);
        self.over_card.set(false);
        if self.shown.replace(None).is_none() {
            return;
        }
        self.closed_at.set(Some(Instant::now()));
        let Some(card) = self.card.borrow().clone() else {
            return;
        };
        if let Some(view) = self.view.upgrade() {
            self.mark(&view, None);
        }
        if !card.popover.settings().is_gtk_enable_animations() {
            self.discard_card();
            return;
        }
        card.popover.add_css_class("closing");
        clear(&self.exit_timer);
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(EXIT, move || {
            if let Some(preview) = weak.upgrade() {
                preview.exit_timer.borrow_mut().take();
                preview.discard_card();
            }
        });
        self.exit_timer.replace(Some(id));
    }
}

#[cfg(test)]
impl LinkPreview {
    pub(super) fn shown_url(&self) -> Option<String> {
        self.shown.borrow().as_ref().map(|(l, _)| l.url.clone())
    }
    pub(super) fn is_pending(&self) -> bool {
        self.pending.borrow().is_some()
    }
    pub(super) fn card_is_visible(&self) -> bool {
        self.card
            .borrow()
            .as_ref()
            .is_some_and(|c| c.popover.is_visible())
    }
    pub(super) fn title(&self) -> String {
        self.card
            .borrow()
            .as_ref()
            .map_or_else(String::new, |c| c.title.text().to_string())
    }
    pub(super) fn click_copy(&self) {
        if let Some(card) = &*self.card.borrow() {
            card.copy.emit_clicked();
        }
    }
    pub(super) fn click_edit(&self) {
        if let Some(card) = &*self.card.borrow() {
            card.edit.emit_clicked();
        }
    }
}
