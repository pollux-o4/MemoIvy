//! FILE-001 파일 화면. 첨부의 사용처를 보여 주고, 다시 쓰기와 사용 안 함 파일의 영구 삭제를 맡는다.
//! 저장·사용 판정은 persistence Attachments가 원본이고 이 화면은 읽기 모델만 그린다.
mod detail;
mod grid;
#[cfg(test)]
pub(in crate::app) mod tests;

use crate::app::{
    shared::header,
    shell::{commands, state::State},
};
use crate::persistence::{AssetEntry, AssetId};
use detail::Detail;
use grid::Grid;
use gtk::{prelude::*, Application, ApplicationWindow, Box as GtkBox, Label, Orientation};
use std::{cell::RefCell, rc::Rc};

pub(in crate::app) struct FilesView {
    window: ApplicationWindow,
    used: gtk::ToggleButton,
    unused: gtk::ToggleButton,
    grid: Grid,
    empty: Label,
    detail: Detail,
    purge: gtk::Button,
    auto: gtk::CheckButton,
    selected: RefCell<Option<AssetId>>,
    entries: RefCell<Vec<AssetEntry>>,
}

/// 하나만 연다. 이미 열려 있으면 앞으로 가져온다.
pub(in crate::app) fn open(state: &State, app: &Application) {
    if let Some(view) = state.borrow().views.files.clone() {
        view.window.present();
        return;
    }
    let view = FilesView::build(state, app);
    state.borrow_mut().views.files = Some(view.clone());
    view.refresh(state);
    view.window.present();
}

impl FilesView {
    fn build(state: &State, app: &Application) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("파일")
            .default_width(720)
            .default_height(520)
            .build();
        crate::app::shared::focus::watch(&window);
        crate::app::shared::close_on_escape(&window);
        header(&window, "파일");

        let used = gtk::ToggleButton::with_label("사용 중");
        let unused = gtk::ToggleButton::with_label("사용 안 함");
        unused.set_group(Some(&used));
        used.set_active(true);
        let tabs = GtkBox::new(Orientation::Horizontal, 0);
        tabs.add_css_class("segmented");
        tabs.append(&used);
        tabs.append(&unused);
        let purge = gtk::Button::with_label("사용 안 함 모두 삭제…");
        purge.add_css_class("memoivy-action");
        purge.add_css_class("destructive");
        let auto = gtk::CheckButton::with_label("30일 지난 항목 자동 정리");
        auto.set_tooltip_text(Some(
            "사용 안 함이 된 지 30일 지난 파일을 앱을 열 때 지워요",
        ));
        auto.add_css_class("memoivy-action");
        let toolbar = GtkBox::new(Orientation::Horizontal, 8);
        toolbar.add_css_class("files-toolbar");
        tabs.set_hexpand(true);
        tabs.set_halign(gtk::Align::Start);
        toolbar.append(&tabs);
        toolbar.append(&auto);
        toolbar.append(&purge);

        let grid = Grid::new();
        let empty = Label::new(None);
        empty.add_css_class("files-empty");
        empty.set_wrap(true);
        empty.set_justify(gtk::Justification::Center);
        empty.set_vexpand(true);
        let list = GtkBox::new(Orientation::Vertical, 0);
        list.set_hexpand(true);
        list.append(&grid.scroll);
        list.append(&empty);
        let detail = Detail::new();
        let body = GtkBox::new(Orientation::Horizontal, 0);
        body.append(&list);
        body.append(&detail.root);
        let outer = GtkBox::new(Orientation::Vertical, 0);
        outer.add_css_class("files");
        outer.append(&toolbar);
        outer.append(&body);
        window.set_child(Some(&outer));

        let view = Rc::new(Self {
            window,
            used,
            unused,
            grid,
            empty,
            detail,
            purge,
            auto,
            selected: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
        });
        view.connect(state, app);
        view
    }

    fn connect(self: &Rc<Self>, state: &State, app: &Application) {
        for tab in [&self.used, &self.unused] {
            let (weak, st) = (Rc::downgrade(self), state.clone());
            tab.connect_toggled(move |tab| {
                if let Some(view) = weak.upgrade().filter(|_| tab.is_active()) {
                    view.selected.replace(None);
                    view.render(&st);
                }
            });
        }
        let (weak, st) = (Rc::downgrade(self), state.clone());
        self.grid.connect_selected(move |id| {
            if let Some(view) = weak.upgrade() {
                view.selected.replace(Some(id));
                view.show_detail(&st);
            }
        });
        let st = state.clone();
        self.auto
            .set_active(state.borrow().attachments.auto_prune());
        self.auto.connect_toggled(move |auto| {
            st.borrow_mut().attachments.set_auto_prune(auto.is_active());
        });
        let (weak, st) = (Rc::downgrade(self), state.clone());
        self.purge.connect_clicked(move |_| {
            let Some(view) = weak.upgrade() else {
                return;
            };
            let count = view
                .entries
                .borrow()
                .iter()
                .filter(|e| e.users.is_empty())
                .count();
            let st = st.clone();
            let refresh = Rc::downgrade(&view);
            crate::app::shared::confirm(
                &view.window,
                &format!("사용 안 함 파일 {count}개를 삭제할까요?"),
                None,
                "삭제한 파일은 복구할 수 없어요. 메모에서 쓰고 있는 파일은 지우지 않아요.",
                "삭제",
                move || {
                    let mut guard = st.borrow_mut();
                    let s = &mut *guard;
                    let result = s.attachments.delete_unused(s.notebook.notes());
                    drop(guard);
                    if let Some(view) = refresh.upgrade() {
                        view.after_delete(&st, result.err());
                    }
                },
            );
        });
        let (weak, st) = (Rc::downgrade(self), state.clone());
        self.detail.connect_delete(move |id| {
            let Some(view) = weak.upgrade() else {
                return;
            };
            let st = st.clone();
            let refresh = Rc::downgrade(&view);
            crate::app::shared::confirm(
                &view.window,
                "이 파일을 삭제할까요?",
                None,
                "삭제한 파일은 복구할 수 없어요.",
                "삭제",
                move || {
                    let mut guard = st.borrow_mut();
                    let s = &mut *guard;
                    let result = s.attachments.delete(&id, s.notebook.notes());
                    drop(guard);
                    if let Some(view) = refresh.upgrade() {
                        view.after_delete(&st, result.err());
                    }
                },
            );
        });
        let (st, a) = (state.clone(), app.clone());
        self.detail
            .connect_open_note(move |note| commands::open_note(&st, &a, note));
        let st = state.clone();
        self.window.connect_close_request(move |_| {
            st.borrow_mut().views.files = None;
            gtk::glib::Propagation::Proceed
        });
    }

    fn after_delete(&self, state: &State, error: Option<String>) {
        self.refresh(state);
        if let Some(error) = error {
            self.detail.notice(&error);
        }
    }

    /// 저장소에서 다시 읽어 그린다. 사용처가 바뀌면 shell이 부른다.
    pub(in crate::app) fn refresh(&self, state: &State) {
        let entries = {
            let s = state.borrow();
            s.attachments.entries(s.notebook.notes())
        };
        let used = entries.iter().filter(|e| !e.users.is_empty()).count();
        self.used.set_label(&format!("사용 중 {used}"));
        self.unused
            .set_label(&format!("사용 안 함 {}", entries.len() - used));
        *self.entries.borrow_mut() = entries;
        self.render(state);
    }

    fn render(&self, state: &State) {
        let showing_used = self.used.is_active();
        let shown: Vec<_> = self
            .entries
            .borrow()
            .iter()
            .filter(|e| e.users.is_empty() != showing_used)
            .cloned()
            .collect();
        self.purge.set_visible(!showing_used && !shown.is_empty());
        self.auto.set_visible(!showing_used);
        self.empty.set_visible(shown.is_empty());
        self.grid.scroll.set_visible(!shown.is_empty());
        self.empty.set_text(if showing_used {
            "메모에 스크린샷을 붙여 넣거나 이미지 파일을 끌어 놓으면 여기에 모여요."
        } else {
            "메모에서 뺀 이미지가 여기에 남아요. 다시 쓰려면 메모로 끌어 놓으세요."
        });
        let selected = self
            .selected
            .borrow()
            .clone()
            .filter(|id| shown.iter().any(|e| &e.id == id))
            .or_else(|| shown.first().map(|e| e.id.clone()));
        self.selected.replace(selected.clone());
        self.grid.set(&shown, selected.as_ref());
        self.show_detail(state);
    }

    fn show_detail(&self, state: &State) {
        let selected = self.selected.borrow().clone();
        let entry =
            selected.and_then(|id| self.entries.borrow().iter().find(|e| e.id == id).cloned());
        let s = state.borrow();
        let title = |id: uuid::Uuid| {
            s.notebook.note(id).map(|note| {
                let title = crate::presentation::markdown::summary(note.text()).title;
                if title.is_empty() {
                    "제목 없는 메모".to_owned()
                } else {
                    title
                }
            })
        };
        self.detail.show(entry.as_ref(), &title);
    }
}
