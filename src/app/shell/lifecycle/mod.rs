//! 저장 이벤트·종료 대기·타이머를 조율하며 기존 화면 계약을 유지한다.
mod closing;
mod events;
#[cfg(test)]
mod tests;
use super::{commands, state::State, views::refresh_all};
use crate::persistence::SaveStatus;
pub(in crate::app) use closing::{close_editor, close_library};
use gtk::{glib, prelude::*};
use std::time::{Duration, Instant};
#[cfg(test)]
pub(in crate::app) use tests::save_002_close_failure_and_idle_timer;

pub(in crate::app) fn mark(state: &State) {
    let mut s = state.borrow_mut();
    s.refresh_at = Some(Instant::now());
    match s.notebook.status() {
        SaveStatus::Saved => {
            for editor in s.views.windows.values() {
                editor.show_status("저장됨", false);
            }
        }
        SaveStatus::Pending | SaveStatus::Saving => {
            for editor in s.views.windows.values() {
                editor.show_status("저장 중…", false);
            }
        }
        SaveStatus::Failed(_) => {}
    }
    drop(s);
    wake(state);
}
pub(in crate::app) fn flush(state: &State) {
    state.borrow_mut().notebook.flush();
    wake(state);
}
pub(in crate::app) fn wake(state: &State) {
    if state.borrow().timer_active {
        return;
    }
    state.borrow_mut().timer_active = true;
    let weak = std::rc::Rc::downgrade(state);
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(state) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        tick(&state)
    });
}
fn tick(state: &State) -> glib::ControlFlow {
    let now = Instant::now();
    let events = state.borrow_mut().notebook.tick(now);
    let app = state.borrow().views.library.window.application();
    let Some(app) = app else {
        state.borrow_mut().timer_active = false;
        return glib::ControlFlow::Break;
    };
    let received = !events.is_empty();
    events::apply(state, &app, events);
    if received {
        // 앞선 삭제의 저장까지 끝난 뒤에만 다음을 시작한다. 그래야 저장 실패에서 남은 삭제가 실제로 멈춘다.
        // 닫기 판단보다 먼저 시작해 대기열이 남았는데 저장됨으로 보이지 않게 한다.
        if matches!(state.borrow().notebook.status(), SaveStatus::Saved) {
            commands::next_delete(state);
        }
        crate::app::canvas::update_status(state);
    }
    closing::settle(state);
    let mut s = state.borrow_mut();
    let refresh = s
        .refresh_at
        .is_some_and(|at| now.duration_since(at) >= Duration::from_millis(180));
    if refresh {
        s.refresh_at = None;
    }
    let keep = s.notebook.needs_tick() || s.refresh_at.is_some();
    s.timer_active = keep;
    drop(s);
    if refresh {
        refresh_all(state, &app);
    }
    if keep {
        glib::ControlFlow::Continue
    } else {
        glib::ControlFlow::Break
    }
}
