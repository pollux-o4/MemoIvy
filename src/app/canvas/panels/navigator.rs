//! 프로젝트 경로와 바로가기. 깊이와 상관없이 고정하거나 자주 가는 프로젝트로 한 번에 건너간다.
use super::super::{port, CanvasView};
use crate::presentation::icons;
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation};
use std::{fmt::Write as _, rc::Rc};
use uuid::Uuid;

/// 고정 + 자동 추천을 합친 칩 수. 많으면 고르는 데 오히려 시간이 든다.
const SHORTCUTS: usize = 5;
const CHIP_CHARS: i32 = 14;

pub(in crate::app::canvas) struct Navigator {
    pub root: GtkBox,
    crumbs: GtkBox,
    shortcuts: GtkBox,
    pin: Button,
}

pub(super) fn build() -> Navigator {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.set_halign(gtk::Align::Start);
    root.set_valign(gtk::Align::Start);
    root.set_margin_start(16);
    root.set_margin_top(16);
    root.add_css_class("canvas-navigator");
    let crumbs = GtkBox::new(Orientation::Horizontal, 2);
    crumbs.add_css_class("canvas-crumbs");
    crumbs.add_css_class("floating");
    let pin = Button::new();
    pin.set_child(Some(&icons::image(icons::PIN)));
    pin.add_css_class("tool");
    pin.add_css_class("small");
    pin.add_css_class("crumb-pin");
    pin.set_widget_name("canvas-control-pin");
    let shortcuts = GtkBox::new(Orientation::Horizontal, 6);
    shortcuts.add_css_class("canvas-shortcuts");
    root.append(&crumbs);
    root.append(&shortcuts);
    root.set_visible(false);
    Navigator {
        root,
        crumbs,
        shortcuts,
        pin,
    }
}

pub(super) fn wire(v: &Rc<CanvasView>) {
    let weak = Rc::downgrade(v);
    v.widgets.navigator.pin.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            if let Some(id) = v.session.scope.get() {
                v.toggle_pin(id);
            }
        }
    });
}

fn jump(v: &Rc<CanvasView>, button: &Button, scope: Option<Uuid>) {
    let weak = Rc::downgrade(v);
    button.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.go_to(scope);
        }
    });
}

fn dot(color: &str) -> GtkBox {
    let dot = GtkBox::new(Orientation::Horizontal, 0);
    dot.add_css_class("color-dot");
    dot.add_css_class(color);
    dot.set_valign(gtk::Align::Center);
    dot
}

/// 경로·바로가기가 바뀐 경우에만 다시 만든다.
pub(super) fn sync(v: &Rc<CanvasView>, compact: bool) {
    let Some(st) = v.state.upgrade() else {
        return;
    };
    let scope = v.session.scope.get();
    let (path, shortcuts, any_project) = {
        let s = st.borrow();
        (
            port::path(&s, scope),
            port::shortcuts(&s, scope, SHORTCUTS),
            v.session.nodes.borrow().iter().any(|n| n.project.is_some()),
        )
    };
    let pinned = scope.is_some_and(|id| port::pinned(&st.borrow(), id));
    let mut key = format!("{compact}{pinned}{any_project}");
    for place in &path {
        let _ = write!(key, "|{}{}{}", place.id, place.title, place.color);
    }
    for (place, pin) in &shortcuts {
        let _ = write!(key, "#{}{}{}{pin}", place.id, place.title, place.color);
    }
    let nav = &v.widgets.navigator;
    // 프로젝트가 하나도 없으면 길 안내도 필요 없다.
    nav.root
        .set_visible(!path.is_empty() || !shortcuts.is_empty() || any_project);
    if *v.session.navigator_key.borrow() == key {
        return;
    }
    *v.session.navigator_key.borrow_mut() = key;
    while let Some(child) = nav.crumbs.first_child() {
        nav.crumbs.remove(&child);
    }
    while let Some(child) = nav.shortcuts.first_child() {
        nav.shortcuts.remove(&child);
    }
    let home = Button::new();
    let contents = GtkBox::new(Orientation::Horizontal, 6);
    contents.append(&icons::image(icons::HOME));
    if path.is_empty() || !compact {
        contents.append(&Label::new(Some("전체")));
    }
    home.set_child(Some(&contents));
    home.add_css_class("crumb");
    home.set_tooltip_text(Some("최상위 보드"));
    home.update_property(&[gtk::accessible::Property::Label("최상위 보드로 이동")]);
    if path.is_empty() {
        home.add_css_class("current");
        home.set_sensitive(false);
    } else {
        jump(v, &home, None);
    }
    nav.crumbs.append(&home);
    // 좁은 창에서는 가운데 경로를 줄여 현재 위치와 바로 위만 남긴다.
    let skip = if compact {
        path.len().saturating_sub(2)
    } else {
        0
    };
    if skip > 0 {
        nav.crumbs.append(&separator());
        let more = Label::new(Some("…"));
        more.add_css_class("crumb-more");
        nav.crumbs.append(&more);
    }
    for (index, place) in path.iter().enumerate().skip(skip) {
        nav.crumbs.append(&separator());
        let current = index + 1 == path.len();
        let crumb = Button::new();
        let contents = GtkBox::new(Orientation::Horizontal, 6);
        contents.append(&dot(&place.color));
        let title = Label::new(Some(&place.title));
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(if current { 22 } else { CHIP_CHARS });
        contents.append(&title);
        crumb.set_child(Some(&contents));
        crumb.add_css_class("crumb");
        crumb.set_tooltip_text(Some(&place.title));
        if current {
            crumb.add_css_class("current");
            crumb.set_tooltip_text(Some("나가기 · Esc 또는 Alt+↑"));
            jump(v, &crumb, None);
            crumb.set_sensitive(false);
        } else {
            jump(v, &crumb, Some(place.id));
        }
        nav.crumbs.append(&crumb);
    }
    if scope.is_some() {
        nav.pin.set_tooltip_text(Some(if pinned {
            "바로가기 고정 해제"
        } else {
            "바로가기에 고정"
        }));
        nav.pin
            .update_property(&[gtk::accessible::Property::Label(if pinned {
                "바로가기 고정 해제"
            } else {
                "바로가기에 고정"
            })]);
        if pinned {
            nav.pin.add_css_class("active");
        } else {
            nav.pin.remove_css_class("active");
        }
        nav.crumbs.append(&nav.pin);
    }
    nav.shortcuts.set_visible(!compact && !shortcuts.is_empty());
    for (place, pin) in shortcuts {
        let chip = Button::new();
        chip.add_css_class("shortcut-chip");
        chip.add_css_class("floating");
        let contents = GtkBox::new(Orientation::Horizontal, 6);
        if pin {
            let star = icons::image(icons::PIN);
            star.add_css_class("chip-pin");
            contents.append(&star);
        } else {
            contents.append(&dot(&place.color));
        }
        let title = Label::new(Some(&place.title));
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(CHIP_CHARS);
        contents.append(&title);
        chip.set_child(Some(&contents));
        chip.set_tooltip_text(Some(&format!(
            "{} · {}",
            place.title,
            if pin {
                "고정한 바로가기"
            } else {
                "자주 방문"
            }
        )));
        jump(v, &chip, Some(place.id));
        nav.shortcuts.append(&chip);
    }
}

fn separator() -> gtk::DrawingArea {
    let chevron = icons::image(icons::CHEVRON);
    chevron.add_css_class("crumb-separator");
    chevron
}
