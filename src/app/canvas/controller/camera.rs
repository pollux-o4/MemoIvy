use super::*;

impl CanvasView {
    pub(in crate::app::canvas) fn keep_camera(&self, camera: Camera) -> Camera {
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        if viewport.0 < 1. || viewport.1 < 1. {
            return camera;
        }
        self.session
            .content
            .get()
            .map_or(camera, |area| camera.kept_near(area, viewport))
    }

    pub(in crate::app::canvas) fn queue_camera(self: &Rc<Self>, camera: Camera) {
        // 입력 좌표는 즉시 누적하고 카드·변환 갱신은 프레임당 한 번만 한다.
        let camera = self.keep_camera(camera);
        if camera == self.session.camera.get() {
            return;
        }
        self.session.camera.set(camera);
        if self.session.camera_queued.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        self.widgets.surface.add_tick_callback(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.session.camera_queued.set(false);
                scene::board::set_camera(&view, view.session.camera.get());
                input::pointer::follow(&view);
            }
            gtk::glib::ControlFlow::Break
        });
    }

    pub(in crate::app::canvas) fn reveal(self: &Rc<Self>) {
        let rects: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        if let Some(area) = layout::bounds(rects) {
            motion::ensure_visible(self, area);
        }
    }

    pub(in crate::app::canvas) fn fit(self: &Rc<Self>) {
        let rects = self
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| n.rect())
            .collect::<Vec<_>>();
        self.fit_rects(rects);
    }

    pub(in crate::app::canvas) fn fit_selection(self: &Rc<Self>) {
        let rects: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        self.fit_rects(rects);
    }

    pub(in crate::app::canvas) fn fit_rects(self: &Rc<Self>, rects: Vec<Rect>) {
        let Some(area) = layout::bounds(rects) else {
            return;
        };
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        motion::camera_to(self, Camera::fitting(area, viewport, FIT_PADDING));
    }

    pub(in crate::app::canvas) fn resized(self: &Rc<Self>) {
        scene::board::set_camera(self, self.session.camera.get());
        panels::sync(self);
    }

    pub(in crate::app::canvas) fn first_layout(self: &Rc<Self>) {
        if self.session.fitted.replace(true) {
            return;
        }
        let rects = self
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| n.rect())
            .collect::<Vec<_>>();
        let selected: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        if let Some(area) = layout::bounds(rects) {
            let mut camera = Camera::fitting(area, viewport, FIT_PADDING);
            if let Some(focus) = layout::bounds(selected) {
                if !camera.shows(focus, viewport, 24.) {
                    camera = Camera::centered_on(focus, viewport, 1.);
                }
            }
            scene::board::set_camera(self, camera);
        }
    }
}
