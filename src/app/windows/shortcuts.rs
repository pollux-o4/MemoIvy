use crate::app::state::State;
use gtk::{gdk, glib, prelude::*, ApplicationWindow};

const HELP: &str = "Ctrl+N   새 메모\nCtrl+F   메모 목록 검색\nCtrl+W   현재 창 닫기\n\nCtrl+B   굵게\nCtrl+I   기울임\nCtrl+Shift+X   취소선\nCtrl+K   링크\nCtrl+1 / Ctrl+2   제목 / 소제목\nCtrl+Shift+7   번호 목록\nCtrl+Shift+8   글머리표\nCtrl+Shift+9   체크 목록\nCtrl+Enter   체크 상태 변경\nCtrl+Shift+M   원문 전환\n\nCtrl+Z / Ctrl+Shift+Z   실행 취소 / 다시 실행\nF1   단축키 안내";

pub(in crate::app) fn help(window: &ApplicationWindow) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .text("단축키")
        .secondary_text(HELP)
        .buttons(gtk::ButtonsType::Close)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

pub(super) fn attach(window: &ApplicationWindow, state: &State) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = window.downgrade();
    let state = std::rc::Rc::downgrade(state);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(window) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        if key == gdk::Key::F1 {
            help(&window);
            return glib::Propagation::Stop;
        }
        let mods = modifiers
            & (gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::SHIFT_MASK
                | gdk::ModifierType::ALT_MASK);
        if mods != gdk::ModifierType::CONTROL_MASK {
            return glib::Propagation::Proceed;
        }
        let Some(state) = state.upgrade() else {
            return glib::Propagation::Proceed;
        };
        match key.to_lower() {
            gdk::Key::n => {
                let add = state.borrow().add.clone();
                add.emit_clicked();
            }
            gdk::Key::f => {
                let (library, search) = {
                    let s = state.borrow();
                    (s.library.clone(), s.search.clone())
                };
                library.present();
                search.grab_focus();
                search.select_region(0, -1);
            }
            gdk::Key::w => window.close(),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    window.add_controller(keys);
}
