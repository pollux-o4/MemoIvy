use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CardRoot;

    #[glib::object_subclass]
    impl ObjectSubclass for CardRoot {
        const NAME: &'static str = "LsmCanvasCard";
        type Type = super::CardRoot;
        type ParentType = gtk::Widget;

        fn class_init(class: &mut Self::Class) {
            class.set_layout_manager_type::<gtk::BoxLayout>();
        }
    }

    impl ObjectImpl for CardRoot {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for CardRoot {}
}

glib::wrapper! {
    /// 카드 자체가 키보드 포커스를 받는 세로 컨테이너.
    /// GtkBox·GtkOverlay는 포커스를 자식에게만 넘기므로 카드 루트로 쓸 수 없다.
    pub struct CardRoot(ObjectSubclass<imp::CardRoot>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl CardRoot {
    pub fn new(spacing: u32) -> Self {
        let root: Self = glib::Object::builder()
            .property("accessible-role", gtk::AccessibleRole::Button)
            .property("focusable", true)
            .build();
        if let Some(layout) = root.layout_manager().and_downcast::<gtk::BoxLayout>() {
            layout.set_orientation(gtk::Orientation::Vertical);
            layout.set_spacing(spacing);
        }
        root
    }

    pub fn append(&self, child: &impl IsA<gtk::Widget>) {
        child.set_parent(self);
    }
}
