use super::super::{port, CanvasView};
use crate::app::{shared::creation, shell::commands};
#[cfg(test)]
pub(in crate::app::canvas) use creation::assert_motion;
use std::rc::Rc;

pub(in crate::app::canvas) fn build(view: &Rc<CanvasView>) -> gtk::MenuButton {
    let trigger = gtk::MenuButton::new();
    let weak = Rc::downgrade(view);
    let permissions = weak.clone();
    creation::configure(
        &trigger,
        "canvas-create",
        move |project| {
            if let Some(v) = weak.upgrade() {
                if let Some(state) = v.state.upgrade() {
                    if project {
                        commands::new_project(&state, v.session.scope.get());
                    } else {
                        commands::new_note(&state, v.session.scope.get());
                    }
                }
            }
        },
        move || {
            permissions
                .upgrade()
                .and_then(|v| {
                    v.state
                        .upgrade()
                        .map(|state| port::creation_allowed(&state.borrow(), v.session.scope.get()))
                })
                .unwrap_or((false, false))
        },
    );
    trigger
}
