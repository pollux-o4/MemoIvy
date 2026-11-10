use gtk::{gdk, glib, graphene, gsk, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

const DURATION_US: f64 = 300_000.;
const COLLAPSED_LINES: usize = 2;

type Measurement = ((i32, u32), (i32, i32));

#[cfg(test)]
pub(in crate::app::library) mod tests;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Preview {
        pub label: gtk::Label,
        pub toggle: RefCell<glib::WeakRef<gtk::Button>>,
        pub expanded: Cell<bool>,
        pub context: RefCell<String>,
        pub progress: Cell<f64>,
        pub tick: RefCell<Option<gtk::TickCallbackId>>,
        pub measurements: RefCell<Vec<Measurement>>,
        #[cfg(test)]
        pub layout_passes: Cell<usize>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Preview {
        const NAME: &'static str = "MemoIvyLibraryPreview";
        type Type = super::Preview;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Preview {
        fn constructed(&self) {
            self.parent_constructed();
            self.label.set_xalign(0.);
            self.label.set_yalign(0.);
            self.label.set_wrap(true);
            self.label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            self.label.add_css_class("preview");
            self.label.set_parent(&*self.obj());
            self.obj().set_overflow(gtk::Overflow::Hidden);
            let weak = self.obj().downgrade();
            self.label.connect_notify_local(None, move |_, _| {
                if let Some(preview) = weak.upgrade() {
                    preview.imp().measurements.borrow_mut().clear();
                }
            });
        }

        fn dispose(&self) {
            if let Some(tick) = self.tick.take() {
                tick.remove();
            }
            self.label.unparent();
        }
    }

    impl WidgetImpl for Preview {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                let minimum = self.label.measure(orientation, -1).0;
                return (minimum, minimum, -1, -1);
            }
            let (collapsed, full) = self.heights(for_size);
            let height = (f64::from(collapsed) + f64::from(full - collapsed) * self.progress.get())
                .round() as i32;
            (height, height, -1, -1)
        }

        fn size_allocate(&self, width: i32, _height: i32, _baseline: i32) {
            let (collapsed, full) = self.heights(width);
            self.label.allocate(width, full, -1, None);
            if let Some(toggle) = self.toggle.borrow().upgrade() {
                toggle.set_visible(full > collapsed);
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let (collapsed, full) = self.heights(obj.width());
            let fade = full > collapsed && self.progress.get() < 1.;
            if fade {
                // Alpha mask follows the card surface, including hover and both themes.
                let height = obj.height() as f32;
                let opaque = gdk::RGBA::new(0., 0., 0., 1.);
                let bottom = gdk::RGBA::new(0., 0., 0., self.progress.get() as f32);
                snapshot.push_mask(gsk::MaskMode::Alpha);
                snapshot.append_linear_gradient(
                    &graphene::Rect::new(0., 0., obj.width() as f32, height),
                    &graphene::Point::new(0., 0.),
                    &graphene::Point::new(0., height),
                    &[
                        gsk::ColorStop::new(0., opaque),
                        gsk::ColorStop::new(0.5, opaque),
                        gsk::ColorStop::new(1., bottom),
                    ],
                );
                snapshot.pop();
            }
            obj.snapshot_child(&self.label, snapshot);
            if fade {
                snapshot.pop();
            }
        }
    }

    impl Preview {
        pub fn heights(&self, width: i32) -> (i32, i32) {
            let key = (width, self.label.pango_context().serial());
            if let Some((_, heights)) = self
                .measurements
                .borrow()
                .iter()
                .find(|(cached, _)| *cached == key)
            {
                return *heights;
            }
            #[cfg(test)]
            self.layout_passes.set(self.layout_passes.get() + 1);
            let layout = self.label.create_pango_layout(Some(&self.label.text()));
            layout.set_attributes(self.label.attributes().as_ref());
            layout.set_width(if width < 0 {
                -1
            } else {
                width.max(1) * gtk::pango::SCALE
            });
            layout.set_wrap(gtk::pango::WrapMode::WordChar);
            let full = self.label.measure(gtk::Orientation::Vertical, width).1;
            let mut iter = layout.iter();
            let mut collapsed = 0;
            for _ in 0..COLLAPSED_LINES {
                let (_, bottom) = iter.line_yrange();
                collapsed = (bottom + gtk::pango::SCALE - 1) / gtk::pango::SCALE;
                if !iter.next_line() {
                    break;
                }
            }
            let heights = (collapsed.min(full), full);
            let mut measurements = self.measurements.borrow_mut();
            // GTK의 height-for-width 이진 탐색 너비도 재사용하며, 창 크기 변경이 이어져도 메모리를 제한한다.
            if measurements.len() == 32 {
                measurements.remove(0);
            }
            measurements.push((key, heights));
            heights
        }
    }
}

glib::wrapper! {
    pub struct Preview(ObjectSubclass<imp::Preview>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Preview {
    fn toggle(&self) {
        let imp = self.imp();
        let expanded = !imp.expanded.get();
        imp.expanded.set(expanded);
        let target = f64::from(expanded);
        let button = imp.toggle.borrow().upgrade().expect("preview toggle");
        super::disclosure::reflect(&button, expanded, &imp.context.borrow());
        if let Some(tick) = imp.tick.take() {
            tick.remove();
        }
        if !self.settings().is_gtk_enable_animations() || !self.is_mapped() {
            imp.progress.set(target);
            self.queue_resize();
            return;
        }
        let from = imp.progress.get();
        let start = self.frame_clock().map_or(0, |clock| clock.frame_time());
        let tick = self.add_tick_callback(move |widget, clock| {
            let t = ((clock.frame_time() - start) as f64 / DURATION_US).clamp(0., 1.);
            let progress = from + (target - from) * (1. - (1. - t).powi(3));
            widget.imp().progress.set(progress);
            widget.queue_resize();
            if t >= 1. || !widget.settings().is_gtk_enable_animations() {
                widget.imp().progress.set(target);
                widget.imp().tick.take();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        imp.tick.replace(Some(tick));
    }
}

pub(super) fn build(text: &str, title: &str) -> (gtk::Widget, gtk::Button) {
    let preview: Preview = glib::Object::new();
    // shortcut: 본문 전체를 카드 생성 때 텍스트로 변환, 대용량 목록 지연이 측정되면 펼칠 때 지연 변환 검토.
    preview.imp().label.set_text(&body(text));
    preview.imp().context.replace(title.to_owned());
    let toggle = super::disclosure::button(title);
    toggle.set_visible(false);
    toggle.update_relation(&[gtk::accessible::Relation::Controls(&[preview.upcast_ref()])]);
    preview.imp().toggle.replace(toggle.downgrade());
    let weak = preview.downgrade();
    toggle.connect_clicked(move |_| {
        if let Some(preview) = weak.upgrade() {
            preview.toggle();
        }
    });
    (preview.upcast(), toggle)
}

fn body(text: &str) -> String {
    let mut title = true;
    let mut fence: Option<&str> = None;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(marker) = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m)) {
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            } else {
                lines.push(line.to_owned());
            }
            continue;
        }
        let plain = if fence.is_some() {
            line.to_owned()
        } else {
            // shortcut: 목록 이미지는 설명으로 표시, 목록에서 이미지 미리보기가 필요해지면 전용 렌더링 검토.
            crate::presentation::markdown::plain_line(line)
        };
        if title {
            if !plain.trim().is_empty() {
                title = false;
            }
        } else {
            lines.push(plain);
        }
    }
    lines.join("\n").trim().to_owned()
}
