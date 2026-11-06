//! 단계 전환. 누른 프로젝트 카드가 그대로 커져 새 단계 화면이 되고, 나올 때는 화면이 그 카드로 줄어든다.
//!
//! - 움직임은 임계 감쇠 스프링 하나(`progress`)에서 모두 나온다. 테두리·내용 교차·뒤 화면 그림자가 같은 시계로 움직인다.
//! - 떠나는 화면은 텍스처 한 장으로 굳힌다. 새 단계는 모션 전에 배치를 끝내고, 모션 중에는 카메라 변환만 바꾼다.
//! - 무거운 첫 프레임이 지나 프레임 간격이 안정된 뒤 시계를 시작해 모션이 건너뛰지 않는다.
use super::super::{model::camera::Camera, CanvasView};
use gtk::{gdk, graphene, gsk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// 스프링 응답 시간(초). 임계 감쇠라 넘치지 않고, 이 시간의 약 1.6배에 눈으로 멈춘다.
const RESPONSE: f64 = 0.42;
/// 새 단계의 첫 배치·그리기가 끝나기를 기다리는 최대 프레임 수.
const WARM_FRAMES: u32 = 4;
/// 뒤 화면을 덮는 그림자의 최대 농도.
const DIM: f32 = 0.12;
/// 카드 모서리 반지름(월드 단위).
const RADIUS: f64 = 14.;
/// 따라갈 카드가 없을 때(경로로 건너뛰기) 쓰는 가운데 상자 비율.
const INSET: f32 = 0.86;

#[derive(Clone, Copy, PartialEq)]
pub(in crate::app::canvas) enum Kind {
    /// 카드 → 화면. `card`는 떠나는 화면의 카드 자리, `camera`는 도착 카메라.
    Enter { camera: Camera },
    /// 화면 → 카드. `card`는 도착 화면의 카드 자리.
    Leave,
}

pub(in crate::app::canvas) struct Dive {
    texture: gdk::Texture,
    kind: Kind,
    card: graphene::Rect,
    /// 실제 카드 자리인지. 아니면(가운데 상자) 카드 표면을 그리지 않는다.
    matched: bool,
    radius: f32,
    progress: f64,
}

/// 보드 배경과 카드 층을 한 장으로 렌더링한다. 위에 뜬 도구 막대는 살아 있는 채로 둔다.
pub(in crate::app::canvas) fn capture(v: &CanvasView) -> Option<gdk::Texture> {
    let (width, height) = size(v);
    if width < 1. || height < 1. {
        return None;
    }
    let scale = v.widgets.backdrop.scale_factor() as f32;
    let snapshot = gtk::Snapshot::new();
    snapshot.scale(scale, scale);
    for widget in [
        v.widgets.backdrop.upcast_ref::<gtk::Widget>(),
        v.widgets.surface.upcast_ref(),
    ] {
        gtk::WidgetPaintable::new(Some(widget)).snapshot(
            &snapshot,
            f64::from(width),
            f64::from(height),
        );
    }
    let node = snapshot.to_node()?;
    let renderer = v.widgets.window.renderer()?;
    Some(renderer.render_texture(
        node,
        Some(&graphene::Rect::new(0., 0., width * scale, height * scale)),
    ))
}

fn size(v: &CanvasView) -> (f32, f32) {
    (
        v.widgets.backdrop.width() as f32,
        v.widgets.backdrop.height() as f32,
    )
}

/// 카드 월드 위치를 주어진 카메라의 화면 상자로 바꾼다. 화면 밖이면 None.
pub(in crate::app::canvas) fn card_box(
    v: &CanvasView,
    camera: Camera,
    rect: super::super::model::layout::Rect,
) -> Option<graphene::Rect> {
    let (x0, y0) = camera.to_screen((rect.x, rect.y));
    let (x1, y1) = camera.to_screen((rect.right(), rect.bottom()));
    let (w, h) = size(v);
    let visible = x1 > 0. && y1 > 0. && x0 < f64::from(w) && y0 < f64::from(h);
    visible.then(|| graphene::Rect::new(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32))
}

/// 따라갈 카드가 없을 때 화면 가운데의 조금 작은 상자에서 시작하거나 끝난다.
fn center_box(v: &CanvasView) -> graphene::Rect {
    let (w, h) = size(v);
    let (iw, ih) = (w * INSET, h * INSET);
    graphene::Rect::new((w - iw) / 2., (h - ih) / 2., iw, ih)
}

/// 임계 감쇠 스프링의 위치(0→1). 속도 0에서 출발해 속도 0으로 끝난다.
fn spring(seconds: f64) -> f64 {
    let t = seconds.max(0.) * std::f64::consts::TAU / RESPONSE;
    1. - (1. + t) * (-t).exp()
}

fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    let t = ((x - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn lerp(a: f32, b: f32, t: f64) -> f32 {
    a + (b - a) * t as f32
}

fn lerp_rect(a: &graphene::Rect, b: &graphene::Rect, t: f64) -> graphene::Rect {
    graphene::Rect::new(
        lerp(a.x(), b.x(), t),
        lerp(a.y(), b.y(), t),
        lerp(a.width(), b.width(), t),
        lerp(a.height(), b.height(), t),
    )
}

/// 화면 전체를 `rect`에 꽉 차게(넘치는 쪽은 잘라) 넣는 배율과 원점.
fn cover(rect: &graphene::Rect, full: (f32, f32)) -> (f32, f32, f32) {
    let s = (rect.width() / full.0).max(rect.height() / full.1);
    let x = rect.x() + (rect.width() - full.0 * s) / 2.;
    let y = rect.y() + (rect.height() - full.1 * s) / 2.;
    (s, x, y)
}

/// 지금 테두리 상자. 들어갈 때 카드 → 화면, 나올 때 화면 → 카드.
fn frame(dive: &Dive, full: (f32, f32), e: f64) -> graphene::Rect {
    let screen = graphene::Rect::new(0., 0., full.0, full.1);
    match dive.kind {
        Kind::Enter { .. } => lerp_rect(&dive.card, &screen, e),
        Kind::Leave => lerp_rect(&screen, &dive.card, e),
    }
}

/// 들어갈 때 새 단계 화면이 테두리 상자 안에 보이도록 도착 카메라를 줄여 옮긴다.
fn inner_camera(target: Camera, rect: &graphene::Rect, full: (f32, f32)) -> Camera {
    let (s, x, y) = cover(rect, full);
    let zoom = target.zoom * f64::from(s);
    Camera {
        zoom,
        x: target.x - f64::from(x) / zoom,
        y: target.y - f64::from(y) / zoom,
    }
}

/// 모션 한 지점을 화면에 반영한다. 프레임 띠 확인과 실제 재생이 같은 함수를 쓴다.
pub(in crate::app::canvas) fn pose(v: &CanvasView, e: f64) {
    let full = size(v);
    let camera = {
        let mut dive = v.session.dive.borrow_mut();
        let Some(dive) = dive.as_mut() else {
            return;
        };
        dive.progress = e;
        match dive.kind {
            Kind::Enter { camera } => Some(inner_camera(camera, &frame(dive, full, e), full)),
            Kind::Leave => None,
        }
    };
    if let Some(camera) = camera {
        super::board::apply_camera(v, camera, false);
    }
    v.widgets.freeze.queue_draw();
}

/// 굳힌 화면을 덮는다. `zoom`은 카드 모서리를 맞출 배율(카드가 보이는 화면의 배율)이다.
pub(in crate::app::canvas) fn begin(
    v: &CanvasView,
    texture: gdk::Texture,
    kind: Kind,
    card: Option<graphene::Rect>,
    zoom: f64,
) {
    *v.session.dive.borrow_mut() = Some(Dive {
        texture,
        kind,
        matched: card.is_some(),
        card: card.unwrap_or_else(|| center_box(v)),
        radius: (RADIUS * zoom) as f32,
        progress: 0.,
    });
    v.widgets.freeze.set_visible(true);
    pose(v, 0.);
}

#[cfg(test)]
thread_local! {
    /// 프레임 띠 확인용. 참이면 재생 시계가 자세를 바꾸지 않아 `pose`로 고른 지점을 그대로 렌더링한다.
    pub(in crate::app::canvas) static HOLD: Cell<bool> = const { Cell::new(false) };
}

/// 첫 무거운 프레임을 굳힌 화면 아래로 보내고, 프레임이 안정되면 스프링을 시작한다.
pub(in crate::app::canvas) fn play(v: &Rc<CanvasView>, done: impl FnOnce() + 'static) {
    let weak = Rc::downgrade(v);
    let done = RefCell::new(Some(done));
    let began = Cell::new(None::<i64>);
    let warm = Cell::new((0, None::<i64>));
    let log = std::env::var_os("MEMOIVY_FRAME_LOG").is_some();
    let gaps = RefCell::new(Vec::<i64>::new());
    v.widgets.freeze.add_tick_callback(move |widget, clock| {
        let Some(v) = weak.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        #[cfg(test)]
        if HOLD.with(Cell::get) {
            return gtk::glib::ControlFlow::Continue;
        }
        let now = clock.frame_time();
        let period = match clock.refresh_info(0) {
            (_, period) if period > 0 => period,
            _ => 16_667,
        };
        let (frames, last) = warm.get();
        warm.set((frames + 1, Some(now)));
        let Some(start) = began.get() else {
            let steady = last.is_some_and(|last| now - last <= period * 5 / 2);
            if frames >= 1 && (steady || frames >= WARM_FRAMES) {
                began.set(Some(now));
            }
            return gtk::glib::ControlFlow::Continue;
        };
        if let Some(last) = last.filter(|last| *last >= start) {
            gaps.borrow_mut().push(now - last);
        }
        let e = spring((now - start) as f64 / 1e6);
        let finished = 1. - e < 5e-4;
        pose(&v, if finished { 1. } else { e });
        if !finished {
            return gtk::glib::ControlFlow::Continue;
        }
        if log {
            report(&gaps.borrow(), period);
        }
        finish(&v);
        widget.set_visible(false);
        if let Some(done) = done.take() {
            done();
        }
        gtk::glib::ControlFlow::Break
    });
}

/// 덮개를 걷고 도착 카메라를 정식으로 적용한다(배율 단계 스타일·화면 밖 카드 숨기기 포함).
pub(in crate::app::canvas) fn finish(v: &CanvasView) {
    let kind = v.session.dive.replace(None).map(|dive| dive.kind);
    v.widgets.freeze.set_visible(false);
    let camera = match kind {
        Some(Kind::Enter { camera }) => camera,
        _ => v.session.camera.get(),
    };
    super::board::set_camera(v, camera);
}

/// 실제 기기에서 전환 프레임을 잰다. `MEMOIVY_FRAME_LOG=1`일 때만 표준 오류로 남긴다.
fn report(gaps: &[i64], period: i64) {
    if gaps.is_empty() {
        return;
    }
    let mut ms: Vec<f64> = gaps.iter().map(|g| *g as f64 / 1000.).collect();
    ms.sort_by(f64::total_cmp);
    let dropped: i64 = gaps
        .iter()
        .map(|g| ((*g as f64 / period as f64).round() as i64 - 1).max(0))
        .sum();
    eprintln!(
        "MemoIvy frames={} refresh={:.1}ms p50={:.1}ms p95={:.1}ms max={:.1}ms dropped={}",
        ms.len(),
        period as f64 / 1000.,
        ms[ms.len() / 2],
        ms[((ms.len() - 1) as f64 * 0.95) as usize],
        ms[ms.len() - 1],
        dropped
    );
}

pub(in crate::app::canvas) fn attach(v: &Rc<CanvasView>) {
    let weak = Rc::downgrade(v);
    v.widgets
        .freeze
        .set_draw_func(move |snapshot, width, height| {
            let Some(v) = weak.upgrade() else {
                return;
            };
            let dive = v.session.dive.borrow();
            if let Some(dive) = dive.as_ref() {
                draw(dive, snapshot, (width as f32, height as f32));
            }
        });
}

fn rounded(rect: &graphene::Rect, radius: f32) -> gsk::RoundedRect {
    let corner = graphene::Size::new(radius, radius);
    gsk::RoundedRect::new(*rect, corner, corner, corner, corner)
}

/// `rect` 바깥 네 띠에만 그린다. 가운데에는 살아 있는 새 화면이 비친다.
fn around(snapshot: &gtk::Snapshot, rect: &graphene::Rect, full: (f32, f32), paint: &dyn Fn()) {
    let (x0, y0) = (rect.x().max(0.), rect.y().max(0.));
    let (x1, y1) = (
        (rect.x() + rect.width()).min(full.0),
        (rect.y() + rect.height()).min(full.1),
    );
    for band in [
        graphene::Rect::new(0., 0., full.0, y0),
        graphene::Rect::new(0., y1, full.0, full.1 - y1),
        graphene::Rect::new(0., y0, x0, y1 - y0),
        graphene::Rect::new(x1, y0, full.0 - x1, y1 - y0),
    ] {
        if band.width() > 0. && band.height() > 0. {
            snapshot.push_clip(&band);
            paint();
            snapshot.pop();
        }
    }
}

/// 텍스처를 `from` 상자가 `to` 상자에 겹치도록 늘려 그린다.
fn mapped(
    snapshot: &gtk::Snapshot,
    texture: &gdk::Texture,
    full: (f32, f32),
    from: &graphene::Rect,
    to: &graphene::Rect,
) {
    snapshot.save();
    snapshot.translate(&graphene::Point::new(to.x(), to.y()));
    snapshot.scale(to.width() / from.width(), to.height() / from.height());
    snapshot.translate(&graphene::Point::new(-from.x(), -from.y()));
    snapshot.append_texture(texture, &graphene::Rect::new(0., 0., full.0, full.1));
    snapshot.restore();
}

fn draw(dive: &Dive, snapshot: &gtk::Snapshot, full: (f32, f32)) {
    let e = dive.progress;
    let rect = frame(dive, full, e);
    let screen = graphene::Rect::new(0., 0., full.0, full.1);
    let shade = |alpha: f32| snapshot.append_color(&gdk::RGBA::new(0., 0., 0., alpha), &screen);
    match dive.kind {
        Kind::Enter { .. } => {
            // 뒤 화면: 카드 중심 쪽으로 살짝 다가오며 어두워진다. 새 화면이 앞으로 나오는 깊이감.
            let push = 1. + 0.05 * e as f32;
            let c = (
                dive.card.x() + dive.card.width() / 2.,
                dive.card.y() + dive.card.height() / 2.,
            );
            let pushed = graphene::Rect::new(
                c.0 - c.0 * push,
                c.1 - c.1 * push,
                full.0 * push,
                full.1 * push,
            );
            around(snapshot, &rect, full, &|| {
                mapped(snapshot, &dive.texture, full, &screen, &pushed);
                shade(DIM * e as f32);
            });
            // 카드 표면: 원래 카드 그림이 테두리를 따라 늘어나며 앞쪽 절반에서 녹아 새 화면이 드러난다.
            let face = if dive.matched {
                1. - smoothstep(0.0, 0.45, e) as f32
            } else {
                0.
            };
            if face > 0. {
                snapshot.push_rounded_clip(&rounded(&rect, lerp(dive.radius, 0., e)));
                snapshot.push_opacity(f64::from(face));
                mapped(snapshot, &dive.texture, full, &dive.card, &rect);
                snapshot.pop();
                snapshot.pop();
            }
        }
        Kind::Leave => {
            // 뒤 화면(살아 있는 상위 단계)은 그림자가 걷히며 드러난다. 카드가 없으면 떠나는 화면이 가운데에서 흐려진다.
            around(snapshot, &rect, full, &|| shade(DIM * (1. - e) as f32));
            // 떠나는 화면이 카드 자리로 줄어들며 뒤쪽 절반에서 녹아 실제 카드가 드러난다.
            let face = if dive.matched {
                1. - smoothstep(0.55, 1.0, e) as f32
            } else {
                1. - smoothstep(0.0, 0.7, e) as f32
            };
            if face > 0. {
                let (s, x, y) = cover(&rect, full);
                let fitted = graphene::Rect::new(x, y, full.0 * s, full.1 * s);
                snapshot.push_rounded_clip(&rounded(&rect, lerp(0., dive.radius, e)));
                snapshot.push_opacity(f64::from(face));
                mapped(snapshot, &dive.texture, full, &screen, &fitted);
                snapshot.pop();
                snapshot.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 임계 감쇠 스프링은 0에서 출발해 넘치지 않고 단조롭게 1에 닿는다. 넘치면 카드 테두리가 화면 밖으로 튄다.
    #[test]
    fn canvas_002_dive_spring_is_monotonic_without_overshoot() {
        assert_eq!(spring(0.), 0.);
        let mut last = 0.;
        for step in 1..=200 {
            let e = spring(f64::from(step) / 100.);
            assert!(e >= last && e <= 1., "step {step}: {e}");
            last = e;
        }
        assert!(
            1. - spring(RESPONSE * 1.6) < 5e-3,
            "settles near the response time"
        );
    }

    /// 도착 카메라를 테두리 상자에 맞추면 화면 전체일 때 원래 카메라와 같다.
    #[test]
    fn canvas_002_dive_inner_camera_matches_target_at_full_screen() {
        let target = Camera {
            x: 120.,
            y: -40.,
            zoom: 0.8,
        };
        let full = (1200., 800.);
        let camera = inner_camera(target, &graphene::Rect::new(0., 0., full.0, full.1), full);
        assert!((camera.x - target.x).abs() < 1e-6 && (camera.y - target.y).abs() < 1e-6);
        assert!((camera.zoom - target.zoom).abs() < 1e-9);
    }
}
