//! 생성 선택의 표면 모션. 생성·허용 여부는 호출자가 주입한다.
use crate::presentation::icons;
use gtk::{glib, prelude::*};
use std::rc::Rc;

mod surface;
#[cfg(test)]
pub(in crate::app) use surface::assert_motion;
use surface::Surface;

fn new_label() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let plus = icons::image(icons::ADD);
    plus.add_css_class("creation-plus");
    row.append(&plus);
    row.append(&gtk::Label::new(Some("New")));
    row
}

pub(in crate::app) fn configure(
    trigger: &gtk::MenuButton,
    name: &str,
    create: impl Fn(bool) + 'static,
    allowed: impl Fn() -> (bool, bool) + 'static,
) {
    let create = Rc::new(create);
    trigger.set_child(Some(&new_label()));
    trigger.set_direction(gtk::ArrowType::None);
    trigger.set_widget_name(name);
    trigger.set_tooltip_text(Some("메모 또는 프로젝트 만들기"));
    trigger.update_property(&[gtk::accessible::Property::Label("새로 만들기")]);
    trigger.add_css_class("creation-trigger");
    let popover = gtk::Popover::new();
    popover.add_css_class("creation-popover");
    popover.set_has_arrow(false);
    popover.set_halign(gtk::Align::End);
    popover.set_position(gtk::PositionType::Bottom);
    let surface: Surface = glib::Object::new();
    surface.add_css_class("creation-surface");
    let popup = popover.downgrade();
    surface.on_collapsed(move || {
        if let Some(popup) = popup.upgrade() {
            popup.popdown();
        }
    });
    let heading = surface.heading();
    heading.add_css_class("creation-heading");
    heading.set_child(Some(&new_label()));
    heading.update_property(&[gtk::accessible::Property::Label("새로 만들기 닫기")]);
    surface.choices().add_css_class("creation-choices");
    let mut rows = Vec::new();
    for (project, label, svg) in [
        (false, "메모", icons::NOTE),
        (true, "프로젝트", icons::PROJECT),
    ] {
        let row = gtk::Button::new();
        row.add_css_class("menu-row");
        row.set_widget_name(if project {
            "create-project"
        } else {
            "create-note"
        });
        row.update_property(&[gtk::accessible::Property::Label(label)]);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        content.append(&icons::image(svg));
        content.append(&gtk::Label::new(Some(label)));
        row.set_child(Some(&content));
        let create = create.clone();
        let popup = popover.downgrade();
        row.connect_clicked(move |_| {
            if let Some(popup) = popup.upgrade() {
                popup.popdown();
            }
            create(project);
        });
        surface.choices().append(&row);
        rows.push(row);
    }
    let weak = surface.downgrade();
    heading.connect_clicked(move |_| {
        if let Some(surface) = weak.upgrade() {
            surface.toggle();
        }
    });
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = surface.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(surface) = weak.upgrade() {
                surface.close();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    popover.add_controller(keys);
    popover.set_child(Some(&surface));
    trigger.set_popover(Some(&popover));
    let first = rows[0].clone();
    popover.connect_map(move |_| {
        first.grab_focus();
    });
    let weak = surface.downgrade();
    let button = trigger.downgrade();
    popover.connect_show(move |popup| {
        let (Some(surface), Some(button)) = (weak.upgrade(), button.upgrade()) else {
            return;
        };
        let allowed = allowed();
        rows[0].set_sensitive(allowed.0);
        rows[1].set_sensitive(allowed.1);
        rows[1]
            .set_tooltip_text((!allowed.1).then_some("이 위치에는 프로젝트를 더 만들 수 없어요"));
        popup.set_offset(0, -button.height());
        surface.open(button.width(), button.height());
    });
    let weak = surface.downgrade();
    popover.connect_closed(move |_| {
        if let Some(surface) = weak.upgrade() {
            surface.reset();
        }
    });
}
