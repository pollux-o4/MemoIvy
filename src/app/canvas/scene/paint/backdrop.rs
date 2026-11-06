use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

type Draw = Box<dyn Fn(&gtk::Snapshot, f64, f64)>;
type Resized = Box<dyn Fn()>;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Backdrop {
        pub draw: RefCell<Option<Draw>>,
        pub resized: RefCell<Option<Resized>>,
        pub size: Cell<(i32, i32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Backdrop {
        const NAME: &'static str = "LsmCanvasBackdrop";
        type Type = super::Backdrop;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Backdrop {}

    impl WidgetImpl for Backdrop {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            if self.size.replace((width, height)) != (width, height) {
                if let Some(resized) = self.resized.borrow().as_ref() {
                    resized();
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            if let Some(draw) = self.draw.borrow().as_ref() {
                draw(
                    snapshot,
                    f64::from(widget.width()),
                    f64::from(widget.height()),
                );
            }
        }
    }
}

glib::wrapper! {
    /// 렌더 노드로 직접 그리는 보드 배경.
    /// DrawingArea는 매 프레임 창 전체 Cairo 표면을 CPU로 칠하고 올려 이동 중 프레임이 밀린다.
    pub struct Backdrop(ObjectSubclass<imp::Backdrop>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Backdrop {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Backdrop {
    pub fn set_draw_func(&self, draw: impl Fn(&gtk::Snapshot, f64, f64) + 'static) {
        *self.imp().draw.borrow_mut() = Some(Box::new(draw));
        self.queue_draw();
    }

    pub fn connect_resized(&self, resized: impl Fn() + 'static) {
        *self.imp().resized.borrow_mut() = Some(Box::new(resized));
    }
}
