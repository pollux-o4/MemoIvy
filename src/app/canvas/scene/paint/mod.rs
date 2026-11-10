//! 보드 배경·격자·연결선·HUD 그리기. 렌더 노드 위젯(backdrop)과 연결선 타일 캐시(edge_tiles)를 함께 둔다.
pub(in crate::app::canvas) mod backdrop;
mod background;
mod edges;
pub(in crate::app::canvas) use background::draw_backdrop;
pub(in crate::app::canvas) use edges::edge_tiles;
mod hud;

use super::super::{input::Interaction, CanvasView};
use gtk::{cairo, prelude::*};
use std::rc::Rc;

pub(super) type Rgba = (f64, f64, f64, f64);

#[allow(deprecated)]
pub(super) fn token(v: &CanvasView, name: &str) -> (f64, f64, f64) {
    let color = v
        .widgets
        .surface
        .style_context()
        .lookup_color(name)
        .expect("installed application color token");
    (
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
    )
}

pub(super) fn rgba(cr: &cairo::Context, c: Rgba) {
    cr.set_source_rgba(c.0, c.1, c.2, c.3);
}

pub(in super::super) fn attach(view: &Rc<CanvasView>) {
    let weak = Rc::downgrade(view);
    view.widgets
        .backdrop
        .set_draw_func(move |snapshot, width, height| {
            if let Some(v) = weak.upgrade() {
                draw_backdrop(&v, snapshot, width, height);
            }
        });
    let weak = Rc::downgrade(view);
    view.widgets.hud.set_draw_func(move |_, cr, _, _| {
        if let Some(v) = weak.upgrade() {
            hud::draw_hud(&v, cr);
        }
    });
    let weak = Rc::downgrade(view);
    view.widgets.backdrop.connect_resized(move || {
        if let Some(v) = weak.upgrade() {
            let weak = Rc::downgrade(&v);
            // 첫 그리기 전에 맞춰 원점에서 튀어 오르는 프레임을 없앤다.
            gtk::glib::idle_add_local_full(gtk::glib::Priority::HIGH_IDLE, move || {
                if let Some(v) = weak.upgrade() {
                    v.first_layout();
                    v.resized();
                }
                gtk::glib::ControlFlow::Break
            });
        }
    });
    if let Some(settings) = gtk::Settings::default() {
        for property in crate::presentation::theme::WATCHED {
            if settings.find_property(property).is_none() {
                continue;
            }
            let weak = view.widgets.backdrop.downgrade();
            settings.connect_notify_local(Some(property), move |_, _| {
                if let Some(area) = weak.upgrade() {
                    area.queue_draw();
                }
            });
        }
    }
}

/// 그릴 것이 없으면 HUD를 숨겨 빈 전체 화면 텍스처를 매 프레임 올리지 않는다. 화면 이동은 HUD에 그릴 것이 없다.
pub(in super::super) fn sync_hud(v: &CanvasView) {
    let active = !matches!(
        *v.session.interaction.borrow(),
        Interaction::Idle | Interaction::Pan { .. }
    ) || !v.session.guides.borrow().is_empty()
        || v.motion.borrow().pulse.is_some();
    v.widgets.hud.set_visible(active);
    if active {
        v.widgets.hud.queue_draw();
    }
}

#[cfg(test)]
pub(in crate::app::canvas) mod tests {
    use super::background::{draw_grid, screen_rect};
    use super::edges::curves::{head, path, point};
    use super::edges::render::draw_edges;
    use super::*;
    fn pixels(
        renderer: &gtk::gsk::CairoRenderer,
        snapshot: gtk::Snapshot,
        size: (i32, i32),
    ) -> Vec<u8> {
        let mut pixels = vec![0; (size.0 * size.1 * 4) as usize];
        if let Some(node) = snapshot.to_node() {
            let area = screen_rect(0., 0., f64::from(size.0), f64::from(size.1));
            renderer
                .render_texture(&node, Some(&area))
                .download(&mut pixels, size.0 as usize * 4);
        }
        pixels
    }

    /// 화면 이동 중 재사용하는 연결선 타일이 직접 그린 곡선과 같은 화면을 만든다.
    pub(in crate::app::canvas) fn canvas_002_edge_tiles_match_direct(v: &CanvasView) {
        let renderer = gtk::gsk::CairoRenderer::new();
        renderer.realize(None).unwrap();
        v.motion.borrow_mut().link = None;
        *v.session.edge_tiles.borrow_mut() = Default::default();
        let size = (640, 400);
        let frame = || {
            let snapshot = gtk::Snapshot::new();
            draw_edges(v, &snapshot, 640., 400., (0., 0., 0., 1.));
            pixels(&renderer, snapshot, size)
        };
        let direct = frame();
        assert!(
            v.session.edge_tiles.borrow().is_empty(),
            "first frame draws directly"
        );
        let tiled = frame();
        assert!(
            !v.session.edge_tiles.borrow().is_empty(),
            "stable frame reuses tiles"
        );
        let changed = frame();
        renderer.unrealize();
        assert!(direct.iter().any(|&p| p != 0), "fixture must show an edge");
        let worst = direct.iter().zip(&tiled).map(|(a, b)| a.abs_diff(*b)).max();
        assert!(
            worst <= Some(2),
            "tiles must match direct drawing: {worst:?}"
        );
        assert_eq!(tiled, changed);
    }

    /// GTK 초기화가 필요해 캔버스 GTK 시나리오에서 호출한다.
    pub(in crate::app::canvas) fn canvas_002_grid_preserves_subpixel_motion() {
        let renderer = gtk::gsk::CairoRenderer::new();
        renderer.realize(None).unwrap();
        let sample = |x| {
            let snapshot = gtk::Snapshot::new();
            draw_grid(
                super::super::super::model::camera::Camera { x, y: 0., zoom: 1. },
                &snapshot,
                64.,
                64.,
                (1., 1., 1., 1.),
            );
            pixels(&renderer, snapshot, (64, 64))
        };
        let (still, moved) = (sample(0.), sample(0.25));
        renderer.unrealize();
        assert!(still.iter().any(|&p| p != 0), "grid tile must paint dots");
        assert_ne!(
            still, moved,
            "slow pan must not snap the grid to whole pixels"
        );
    }

    #[test]
    fn canvas_001_curve_endpoints_and_finite_paths() {
        for target in [[400., 0.], [0., 400.], [0., 0.], [-400., 0.], [0., -400.]] {
            let p = path([0., 0.], target);
            assert_eq!(point(p, 0.), p[0]);
            assert_eq!(point(p, 1.), p[3]);
            for i in 0..100 {
                let (x, y) = point(p, i as f64 / 100.);
                assert!(x.is_finite() && y.is_finite());
            }
            let half = head(p, 0.5);
            let end = point(p, 0.5);
            assert!((half[3].0 - end.0).abs() < 1e-9 && (half[3].1 - end.1).abs() < 1e-9);
        }
    }
}
