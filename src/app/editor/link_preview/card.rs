use super::*;

pub(super) struct Card {
    pub(super) popover: gtk::Popover,
    pub(super) badge: Label,
    pub(super) title: Label,
    pub(super) address: Label,
    pub(super) warning: Label,
    pub(super) open: Button,
    pub(super) copy: Button,
    pub(super) edit: Button,
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

impl LinkPreview {
    pub(super) fn card(self: &Rc<Self>, view: &TextView) -> Rc<Card> {
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
                preview.card_pointer(true);
                clear(&preview.hide_timer);
            }
        });
        let weak = Rc::downgrade(self);
        hover.connect_leave(move |_| {
            if let Some(preview) = weak.upgrade() {
                preview.card_pointer(false);
                preview.schedule_hide();
            }
        });
        popover.add_controller(hover);
        let weak = Rc::downgrade(self);
        open.connect_clicked(move |_| {
            if let Some(preview) = weak.upgrade() {
                let link = preview.shown().map(|active| active.link);
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
            let link = preview.shown().map(|active| active.link);
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
            let link = preview.shown().map(|active| active.link);
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

    pub(super) fn reset_copy(&self) {
        if let Some(card) = &*self.card.borrow() {
            card.copy.set_label("주소 복사");
            card.copy.remove_css_class("confirmed");
        }
    }

    pub(super) fn fill(&self, card: &Card, view: &TextView, link: &Link) {
        let buffer = view.buffer();
        let label = super::super::source_slice(&buffer, link.range.clone());
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
}
