//! 링크 위에 잠시 머물면 실제 목적지를 보여 주는 떠 있는 카드.
//! 포커스를 가져가지 않아 입력·IME가 끊기지 않고, 네트워크 요청은 하지 않는다.
use crate::presentation::markdown::{self, Link};
use gtk::{gdk, glib, pango, prelude::*, Box as GtkBox, Button, Label, Orientation, TextView};
mod card;
use card::Card;

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

#[derive(Clone)]
struct ActiveCard {
    link: Link,
    source: Source,
    over_card: bool,
}

#[derive(Clone)]
enum CardState {
    Idle,
    // 옆 링크를 기다리는 동안 기존 카드는 계속 보인다.
    Pending {
        link: Link,
        source: Source,
        anchor: gdk::Rectangle,
        shown: Option<ActiveCard>,
    },
    Shown(ActiveCard),
    Leaving,
}

pub(super) struct LinkPreview {
    view: glib::WeakRef<TextView>,
    card: RefCell<Option<Rc<Card>>>,
    state: RefCell<CardState>,
    show_timer: RefCell<Option<glib::SourceId>>,
    hide_timer: RefCell<Option<glib::SourceId>>,
    exit_timer: RefCell<Option<glib::SourceId>>,
    copied_timer: RefCell<Option<glib::SourceId>>,
    closed_at: Cell<Option<Instant>>,
    /// Esc로 닫은 링크. 커서가 그 안에서 움직여도 다시 띄우지 않는다.
    dismissed: RefCell<Option<std::ops::Range<i32>>>,
    hover_tag: gtk::TextTag,
}

fn clear(timer: &RefCell<Option<glib::SourceId>>) {
    if let Some(id) = timer.borrow_mut().take() {
        id.remove();
    }
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
            state: RefCell::new(CardState::Idle),
            show_timer: RefCell::new(None),
            hide_timer: RefCell::new(None),
            exit_timer: RefCell::new(None),
            copied_timer: RefCell::new(None),
            closed_at: Cell::new(None),
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

    fn transition(&self, state: CardState) {
        self.state.replace(state);
    }

    fn shown(&self) -> Option<ActiveCard> {
        match &*self.state.borrow() {
            CardState::Shown(active)
            | CardState::Pending {
                shown: Some(active),
                ..
            } => Some(active.clone()),
            _ => None,
        }
    }

    fn pending(&self) -> Option<(Link, Source)> {
        match &*self.state.borrow() {
            CardState::Pending { link, source, .. } => Some((link.clone(), *source)),
            _ => None,
        }
    }

    fn cancel_pending(&self) {
        clear(&self.show_timer);
        let shown = self.shown();
        self.transition(shown.map_or_else(
            || {
                if self.exit_timer.borrow().is_some() {
                    CardState::Leaving
                } else {
                    CardState::Idle
                }
            },
            CardState::Shown,
        ));
    }

    fn card_pointer(&self, inside: bool) {
        match &mut *self.state.borrow_mut() {
            CardState::Shown(active)
            | CardState::Pending {
                shown: Some(active),
                ..
            } => active.over_card = inside,
            _ => {}
        }
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
        self.transition(CardState::Idle);
        if let Some(view) = self.view.upgrade() {
            self.mark(&view, None);
        }
        if let Some(card) = self.card.borrow_mut().take() {
            card.popover.unparent();
        }
    }

    /// 카드가 가리킬 자리. 숨긴 문법이 있는 줄은 글자 가로 위치가 믿을 만하지 않아 세로만 글자에서 읽는다.
    pub(super) fn anchor(view: &TextView, offset: i32, x: Option<i32>) -> gdk::Rectangle {
        let spot = view.iter_location(&view.buffer().iter_at_offset(offset));
        let (left, y) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, spot.x(), spot.y());
        gdk::Rectangle::new(x.unwrap_or(left), y, 1, spot.height())
    }

    /// 포인터가 링크 위에 있다. 같은 링크를 재예약하지 않는다.
    pub(super) fn hover(self: &Rc<Self>, link: &Link, anchor: gdk::Rectangle) {
        if let Some(mut active) = self
            .shown()
            .filter(|active| active.link.owner == link.owner)
        {
            clear(&self.hide_timer);
            clear(&self.show_timer);
            active.source = Source::Pointer;
            self.transition(CardState::Shown(active));
            return;
        }
        clear(&self.hide_timer);
        if self.pending().is_some_and(|(pending, source)| {
            pending.owner == link.owner && source == Source::Pointer
        }) {
            return;
        }
        self.schedule_show(link.clone(), Source::Pointer, anchor, HOVER_DELAY);
    }

    pub(super) fn pointer_left(self: &Rc<Self>) {
        if self
            .pending()
            .is_some_and(|(_, source)| source == Source::Pointer)
        {
            self.cancel_pending();
        }
        if self
            .shown()
            .is_some_and(|active| active.source == Source::Pointer)
        {
            self.schedule_hide();
        }
    }

    /// 커서가 링크 안에 있다. Esc로 닫은 같은 링크는 다시 예약하지 않는다.
    pub(super) fn caret(self: &Rc<Self>, link: &Link, anchor: gdk::Rectangle) {
        if self.dismissed.borrow().as_ref() != Some(&link.owner) {
            self.dismissed.replace(None);
        }
        if self.dismissed.borrow().as_ref() == Some(&link.owner)
            || self
                .shown()
                .is_some_and(|active| active.link.owner == link.owner)
            || self
                .pending()
                .is_some_and(|(pending, _)| pending.owner == link.owner)
        {
            return;
        }
        self.schedule_show(link.clone(), Source::Caret, anchor, CARET_DELAY);
    }

    pub(super) fn caret_left(self: &Rc<Self>) {
        self.dismissed.replace(None);
        if self
            .pending()
            .is_some_and(|(_, source)| source == Source::Caret)
        {
            self.cancel_pending();
        }
        if self
            .shown()
            .is_some_and(|active| active.source == Source::Caret)
        {
            self.hide();
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
        let recent = self.shown().is_some()
            || self
                .closed_at
                .get()
                .is_some_and(|at| at.elapsed() < SWITCH_WINDOW);
        let delay = if recent && source == Source::Pointer {
            SWITCH_DELAY
        } else {
            delay
        };
        let shown = self.shown();
        self.transition(CardState::Pending {
            link,
            source,
            anchor,
            shown,
        });
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(delay, move || {
            if let Some(preview) = weak.upgrade() {
                preview.show_timer.borrow_mut().take();
                preview.show_pending();
            }
        });
        self.show_timer.replace(Some(id));
    }

    fn show_pending(self: &Rc<Self>) {
        let pending = self.state.borrow().clone();
        let CardState::Pending {
            link,
            source,
            anchor,
            ..
        } = pending
        else {
            return;
        };
        self.cancel_pending();
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
        let over_card = self.shown().is_some_and(|active| active.over_card);
        self.transition(CardState::Shown(ActiveCard {
            link,
            source,
            over_card,
        }));
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
        if self.shown().is_none_or(|active| active.over_card) {
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
        if matches!(*self.state.borrow(), CardState::Leaving) {
            self.transition(CardState::Idle);
        }
        clear(&self.copied_timer);
        if let Some(card) = self.card.borrow_mut().take() {
            card.popover.popdown();
            card.popover.unparent();
        }
    }

    /// 사용자가 직접 닫았다. 같은 링크 안에 있는 동안에는 다시 띄우지 않는다.
    pub(super) fn dismiss(self: &Rc<Self>) {
        let owner = self.shown().map(|active| active.link.owner);
        self.dismissed.replace(owner);
        self.hide();
    }

    pub(super) fn is_shown(&self) -> bool {
        self.shown().is_some()
    }

    /// 대기·표시 중인 카드를 모두 닫는다. 입력·Esc·스크롤처럼 즉시 물러나야 할 때 쓴다.
    pub(super) fn hide(self: &Rc<Self>) {
        clear(&self.show_timer);
        clear(&self.hide_timer);
        let shown = self.shown().is_some();
        self.cancel_pending();
        if !shown {
            return;
        }
        self.transition(CardState::Leaving);
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
        self.shown().map(|active| active.link.url)
    }
    pub(super) fn is_pending(&self) -> bool {
        self.pending().is_some()
    }
    pub(super) fn card_is_mapped(&self) -> bool {
        self.card
            .borrow()
            .as_ref()
            .is_some_and(|c| c.popover.is_mapped())
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
