use super::layout::Rect;

pub(in crate::app::windows::canvas) const MIN_ZOOM: f64 = 0.05;
pub(in crate::app::windows::canvas) const MAX_ZOOM: f64 = 2.;
const KEEP_VISIBLE: f64 = 96.;

/// `x`·`y`는 뷰포트 왼쪽 위에 놓인 월드 좌표다.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::app::windows::canvas) struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            zoom: 1.,
        }
    }
}

impl Camera {
    pub fn to_world(self, screen: (f64, f64)) -> (f64, f64) {
        (screen.0 / self.zoom + self.x, screen.1 / self.zoom + self.y)
    }

    pub fn to_screen(self, world: (f64, f64)) -> (f64, f64) {
        (
            (world.0 - self.x) * self.zoom,
            (world.1 - self.y) * self.zoom,
        )
    }

    /// 앵커 아래의 월드 지점을 고정한 채 배율을 바꾼다.
    pub fn zoomed_at(self, anchor: (f64, f64), zoom: f64) -> Self {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let world = self.to_world(anchor);
        Self {
            x: world.0 - anchor.0 / zoom,
            y: world.1 - anchor.1 / zoom,
            zoom,
        }
    }

    pub fn panned(self, screen_dx: f64, screen_dy: f64) -> Self {
        Self {
            x: self.x + screen_dx / self.zoom,
            y: self.y + screen_dy / self.zoom,
            zoom: self.zoom,
        }
    }

    /// 사각형 전체가 여백 안에 들어오도록 중앙에 맞춘다. 작은 보드는 100%를 넘기지 않는다.
    pub fn fitting(bounds: Rect, viewport: (f64, f64), padding: f64) -> Self {
        let width = (viewport.0 - padding * 2.).max(1.);
        let height = (viewport.1 - padding * 2.).max(1.);
        let zoom = (width / bounds.w.max(1.))
            .min(height / bounds.h.max(1.))
            .clamp(MIN_ZOOM, 1.);
        Self::centered_on(bounds, viewport, zoom)
    }

    pub fn centered_on(bounds: Rect, viewport: (f64, f64), zoom: f64) -> Self {
        Self {
            x: bounds.x + bounds.w / 2. - viewport.0 / zoom / 2.,
            y: bounds.y + bounds.h / 2. - viewport.1 / zoom / 2.,
            zoom,
        }
    }

    /// 사각형이 여백 안에 완전히 보이는지 확인한다.
    pub fn shows(self, rect: Rect, viewport: (f64, f64), padding: f64) -> bool {
        let (left, top) = self.to_screen((rect.x, rect.y));
        let (right, bottom) = self.to_screen((rect.x + rect.w, rect.y + rect.h));
        left >= padding
            && top >= padding
            && right <= viewport.0 - padding
            && bottom <= viewport.1 - padding
    }

    /// 무한 팬으로 내용을 잃어버리지 않도록 내용 일부를 항상 화면에 남긴다.
    pub fn kept_near(self, content: Rect, viewport: (f64, f64)) -> Self {
        let keep = KEEP_VISIBLE / self.zoom;
        let (view_w, view_h) = (viewport.0 / self.zoom, viewport.1 / self.zoom);
        Self {
            x: within(
                self.x,
                content.x - view_w + keep,
                content.x + content.w - keep,
            ),
            y: within(
                self.y,
                content.y - view_h + keep,
                content.y + content.h - keep,
            ),
            zoom: self.zoom,
        }
    }
}

/// 뷰포트가 여백보다 작아 범위가 뒤집혀도 패닉 없이 가운데를 쓴다.
fn within(value: f64, low: f64, high: f64) -> f64 {
    if low > high {
        (low + high) / 2.
    } else {
        value.clamp(low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_002_zoom_keeps_the_point_under_the_anchor() {
        let camera = Camera {
            x: 120.,
            y: -40.,
            zoom: 0.8,
        };
        let anchor = (300., 200.);
        let before = camera.to_world(anchor);
        let after = camera.zoomed_at(anchor, 1.6);
        let world = after.to_world(anchor);
        assert!((world.0 - before.0).abs() < 1e-9 && (world.1 - before.1).abs() < 1e-9);
        assert_eq!(camera.zoomed_at(anchor, 99.).zoom, MAX_ZOOM);
        assert_eq!(camera.zoomed_at(anchor, 0.).zoom, MIN_ZOOM);
    }

    #[test]
    fn canvas_002_fit_centers_content_without_upscaling() {
        let far_away = Rect::new(5000., 5000., 240., 156.);
        let camera = Camera::fitting(far_away, (1000., 700.), 64.);
        assert_eq!(camera.zoom, 1.);
        assert!(camera.shows(far_away, (1000., 700.), 64.));
        let (x, y) = camera.to_screen((5120., 5078.));
        assert!((x - 500.).abs() < 1e-9 && (y - 350.).abs() < 1e-9);
        let huge = Rect::new(0., 0., 20_000., 20_000.);
        let camera = Camera::fitting(huge, (1000., 700.), 64.);
        assert!(camera.zoom < 0.05 + 1e-9 && camera.zoom >= MIN_ZOOM);
    }

    #[test]
    fn canvas_002_pan_cannot_lose_the_content() {
        let content = Rect::new(0., 0., 1000., 800.);
        let lost = Camera {
            x: 50_000.,
            y: -50_000.,
            zoom: 1.,
        }
        .kept_near(content, (800., 600.));
        assert_eq!(lost.x, 1000. - 96.);
        assert_eq!(lost.y, 96. - 600.);
        let tiny = Camera {
            x: 0.,
            y: 0.,
            zoom: 0.05,
        }
        .kept_near(Rect::new(0., 0., 240., 156.), (40., 30.));
        assert!(
            tiny.x.is_finite() && tiny.y.is_finite(),
            "tiny windows never panic"
        );
    }
}
