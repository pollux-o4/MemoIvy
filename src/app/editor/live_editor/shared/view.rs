use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::RefCell;

type Draw = Box<dyn Fn(&gtk::Snapshot)>;
type Allocate = Box<dyn Fn()>;
mod imp {
    use super::*;
    #[derive(Default)]
    pub struct EditorView {
        pub draw: RefCell<Option<Draw>>,
        pub allocate: RefCell<Option<Allocate>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for EditorView {
        const NAME: &'static str = "MemoIvyEditorView";
        type Type = super::EditorView;
        type ParentType = gtk::TextView;
    }
    impl ObjectImpl for EditorView {}
    impl TextViewImpl for EditorView {}
    impl WidgetImpl for EditorView {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            if let Some(allocate) = self.allocate.borrow().as_ref() {
                allocate();
            }
        }
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.parent_snapshot(snapshot);
            if let Some(draw) = self.draw.borrow().as_ref() {
                draw(snapshot);
            }
        }
    }
}

glib::wrapper! {
    pub struct EditorView(ObjectSubclass<imp::EditorView>)
        @extends gtk::TextView, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

pub(super) fn new() -> gtk::TextView {
    glib::Object::new::<EditorView>().upcast()
}

pub(super) fn set_snapshot(view: &gtk::TextView, draw: impl Fn(&gtk::Snapshot) + 'static) {
    let view = view
        .downcast_ref::<EditorView>()
        .expect("live editor requires new_view()");
    view.imp().draw.replace(Some(Box::new(draw)));
}

pub(super) fn set_allocate(view: &gtk::TextView, allocate: impl Fn() + 'static) {
    let view = view
        .downcast_ref::<EditorView>()
        .expect("live editor requires new_view()");
    view.imp().allocate.replace(Some(Box::new(allocate)));
}
