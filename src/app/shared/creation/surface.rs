use gtk::{gdk, glib, graphene, gsk, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

const DURATION_US: f64 = 280_000.;

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct Surface {
        pub heading: gtk::Button,
        pub choices: gtk::Box,
        pub collapsed: RefCell<Option<Box<dyn Fn()>>>,
        pub origin: Cell<(i32, i32)>,
        pub progress: Cell<f64>,
        pub expanded: Cell<bool>,
        pub tick: RefCell<Option<gtk::TickCallbackId>>,
        #[cfg(test)]
        pub allocations: Cell<usize>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Surface {
        const NAME: &'static str = "MemoIvyCreationSurface";
        type Type = super::Surface;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for Surface {
        fn constructed(&self) {
            self.parent_constructed();
            self.origin.set((82, 38));
            self.heading.set_parent(&*self.obj());
            self.choices.set_orientation(gtk::Orientation::Vertical);
            self.choices.set_parent(&*self.obj());
        }
        fn dispose(&self) {
            if let Some(tick) = self.tick.take() {
                tick.remove();
            }
            self.heading.unparent();
            self.choices.unparent();
        }
    }
    impl WidgetImpl for Surface {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let size = if orientation == gtk::Orientation::Horizontal {
                190.max(self.origin.get().0)
                    .max(self.choices.measure(orientation, -1).1)
            } else {
                self.origin.get().1 + self.choices.measure(orientation, 190).1
            };
            (size, size, -1, -1)
        }
        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            #[cfg(test)]
            self.allocations.set(self.allocations.get() + 1);
            let (w, h) = self.origin.get();
            self.heading.allocate(
                w,
                h,
                -1,
                Some(
                    gsk::Transform::new().translate(&graphene::Point::new((width - w) as f32, 0.)),
                ),
            );
            self.choices.allocate(
                width,
                (height - h).max(0),
                -1,
                Some(gsk::Transform::new().translate(&graphene::Point::new(0., h as f32))),
            );
        }
        #[allow(deprecated)] // GTK 4.10~4.18에서 앱의 CSS 색상 토큰을 읽는 API.
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let p = self.progress.get() as f32;
            let (ow, oh) = self.origin.get();
            let w = ow as f32 + (obj.width() - ow) as f32 * p;
            let h = oh as f32 + (obj.height() - oh) as f32 * p;
            let bounds = graphene::Rect::new(obj.width() as f32 - w, 0., w, h);
            let rounded = gsk::RoundedRect::from_rect(bounds, 9. + 5. * p);
            let context = obj.style_context();
            let accent = context
                .lookup_color("memoivy_accent")
                .expect("installed application color token");
            let raised = context
                .lookup_color("memoivy_raised")
                .expect("installed application color token");
            let color = gdk::RGBA::new(
                accent.red() + (raised.red() - accent.red()) * p,
                accent.green() + (raised.green() - accent.green()) * p,
                accent.blue() + (raised.blue() - accent.blue()) * p,
                1.,
            );
            let mut shadow = context
                .lookup_color("memoivy_shadow")
                .expect("installed application color token");
            shadow.set_alpha(shadow.alpha() * p);
            snapshot.append_outset_shadow(&rounded, &shadow, 0., 4., 0., 12.);
            snapshot.push_rounded_clip(&rounded);
            snapshot.append_color(&color, &bounds);
            obj.snapshot_child(&self.heading, snapshot);
            snapshot.save();
            snapshot.translate(&graphene::Point::new(obj.width() as f32 - w, 0.));
            snapshot.push_opacity(f64::from(p * p));
            obj.snapshot_child(&self.choices, snapshot);
            snapshot.pop();
            snapshot.restore();
            snapshot.pop();
        }
    }
}
glib::wrapper! {
    pub struct Surface(ObjectSubclass<imp::Surface>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Surface {
    pub(super) fn heading(&self) -> gtk::Button {
        self.imp().heading.clone()
    }
    pub(super) fn choices(&self) -> gtk::Box {
        self.imp().choices.clone()
    }
    pub(super) fn on_collapsed(&self, callback: impl Fn() + 'static) {
        self.imp().collapsed.replace(Some(Box::new(callback)));
    }
    pub(super) fn open(&self, width: i32, height: i32) {
        self.reset();
        self.imp().origin.set((width, height));
        self.queue_resize();
        self.animate(true);
    }
    pub(super) fn close(&self) {
        self.animate(false);
    }
    pub(super) fn toggle(&self) {
        self.animate(!self.imp().expanded.get());
    }
    pub(super) fn reset(&self) {
        let imp = self.imp();
        if let Some(tick) = imp.tick.take() {
            tick.remove();
        }
        imp.progress.set(0.);
        imp.expanded.set(false);
        imp.choices.set_can_target(false);
        self.remove_css_class("expanded");
    }
    fn animate(&self, expanded: bool) {
        let imp = self.imp();
        imp.expanded.set(expanded);
        imp.choices.set_can_target(false);
        if let Some(tick) = imp.tick.take() {
            tick.remove();
        }
        if expanded {
            self.add_css_class("expanded");
        } else {
            self.remove_css_class("expanded");
        }
        let target = f64::from(expanded);
        if !self.settings().is_gtk_enable_animations() {
            imp.progress.set(target);
            imp.choices.set_can_target(expanded);
            self.queue_draw();
            if !expanded {
                if let Some(collapsed) = imp.collapsed.borrow().as_ref() {
                    collapsed();
                }
            }
            return;
        }
        let from = imp.progress.get();
        let start = Cell::new(None);
        let tick = self.add_tick_callback(move |surface, clock| {
            let first = *start.get().get_or_insert(clock.frame_time());
            start.set(Some(first));
            let t = ((clock.frame_time() - first) as f64 / DURATION_US).clamp(0., 1.);
            surface
                .imp()
                .progress
                .set(from + (target - from) * (1. - (1. - t).powi(3)));
            surface.queue_draw();
            if t >= 1. || !surface.settings().is_gtk_enable_animations() {
                surface.imp().progress.set(target);
                surface.imp().choices.set_can_target(expanded);
                surface.imp().tick.take();
                if !expanded {
                    if let Some(collapsed) = surface.imp().collapsed.borrow().as_ref() {
                        collapsed();
                    }
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        imp.tick.replace(Some(tick));
    }
}

#[cfg(test)]
pub(in crate::app) fn assert_motion(menu: &gtk::MenuButton) {
    use crate::app::test_support::until;
    let popup = menu.popover().unwrap();
    let surface = popup.child().and_downcast::<Surface>().unwrap();
    let settings = menu.settings();
    let original = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(true);
    menu.popup();
    until(|| surface.imp().progress.get() > 0. && surface.imp().progress.get() < 1.);
    let mid = surface.imp().progress.get();
    surface.imp().heading.emit_clicked();
    assert_eq!(
        surface.imp().progress.get(),
        mid,
        "retarget from current shape"
    );
    until(|| !popup.is_mapped());
    assert!(surface.imp().tick.borrow().is_none());
    menu.popup();
    until(|| surface.imp().progress.get() == 1. && surface.imp().tick.borrow().is_none());
    assert!(surface.imp().choices.can_target());
    popup.popdown();
    // 실제 콜백 간격을 기록한다. Xvfb 결과는 GPU 표시 프레임 보장이 아닌 회귀 지표다.
    let samples = std::rc::Rc::new(RefCell::new(Vec::<f64>::new()));
    let last = std::rc::Rc::new(Cell::new(None::<std::time::Instant>));
    let log = samples.clone();
    let previous = last.clone();
    let observer = surface.add_tick_callback(move |surface, _| {
        if surface.imp().tick.borrow().is_some() {
            let now = std::time::Instant::now();
            if let Some(before) = previous.replace(Some(now)) {
                log.borrow_mut()
                    .push(now.duration_since(before).as_secs_f64() * 1000.);
            }
        }
        glib::ControlFlow::Continue
    });
    for _ in 0..4 {
        last.set(None);
        menu.popup();
        until(|| surface.imp().progress.get() > 0. && surface.imp().progress.get() < 1.);
        let allocated = surface.imp().allocations.get();
        until(|| surface.imp().progress.get() == 1. && surface.imp().tick.borrow().is_none());
        assert_eq!(
            surface.imp().allocations.get(),
            allocated,
            "morph must redraw without per-frame layout"
        );
        last.set(None);
        surface.close();
        until(|| !popup.is_mapped());
    }
    observer.remove();
    let mut gaps = samples.borrow().clone();
    assert!(gaps.len() >= 40, "capture several complete transitions");
    gaps.sort_by(f64::total_cmp);
    let p95 = gaps[(gaps.len() - 1) * 95 / 100];
    let fps = 1000. * gaps.len() as f64 / gaps.iter().sum::<f64>();
    println!(
        "PROJECT-003 morph callback cadence: samples={} fps={fps:.1} p95={p95:.2}ms max={:.2}ms",
        gaps.len(),
        gaps[gaps.len() - 1]
    );
    assert!(
        p95 < 33.4,
        "morph has sustained missed 60Hz frames: p95={p95:.2}ms"
    );
    settings.set_gtk_enable_animations(false);
    menu.popup();
    assert_eq!(surface.imp().progress.get(), 1.);
    assert!(surface.imp().tick.borrow().is_none());
    surface.imp().heading.emit_clicked();
    assert_eq!(surface.imp().progress.get(), 0.);
    until(|| !popup.is_mapped());
    settings.set_gtk_enable_animations(original);
}
