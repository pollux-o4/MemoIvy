use super::super::{
    model::{camera::Camera, layout},
    motion, port, CanvasView,
};
use super::card::{Card, PORT_SIZE};
use gtk::prelude::*;
use std::{collections::HashSet, rc::Rc};
use uuid::Uuid;

const FAR_ZOOM: f64 = 0.45;
const TINY_ZOOM: f64 = 0.2;
const CULL_MARGIN: f64 = 48.;
const PORT_MIN_ZOOM: f64 = 0.3;

/// 월드 전체에 변환 하나만 적용해 확대·이동 비용을 카드 수와 무관하게 유지한다.
pub(in super::super) fn set_camera(v: &CanvasView, camera: Camera) {
    apply_camera(v, v.keep_camera(camera), true);
}

/// 범위 제한 없이 카메라를 그대로 적용한다. 단계 전환은 화면 밖 위치를 거쳐 가므로 제한하지 않고,
/// 배율 단계 스타일도 바꾸지 않는다(카드 전체 스타일 재계산이 모션 프레임을 밀어낸다).
pub(in super::super) fn apply_camera(v: &CanvasView, camera: Camera, restyle: bool) {
    v.session.camera.set(camera);
    let transform = gtk::gsk::Transform::new()
        .translate(&gtk::graphene::Point::new(
            (-camera.x * camera.zoom) as f32,
            (-camera.y * camera.zoom) as f32,
        ))
        .scale(camera.zoom as f32, camera.zoom as f32);
    v.widgets
        .stage
        .set_child_transform(&v.widgets.world, Some(&transform));
    v.widgets
        .zoom_label
        .set_label(&format!("{:.0}%", camera.zoom * 100.));
    for (class, limit) in [("far", FAR_ZOOM), ("tiny", TINY_ZOOM)]
        .into_iter()
        .filter(|_| restyle)
    {
        if camera.zoom < limit {
            v.widgets.surface.add_css_class(class);
        } else {
            v.widgets.surface.remove_css_class(class);
        }
    }
    cull(v);
    update_port(v);
    v.widgets.backdrop.queue_draw();
    super::paint::sync_hud(v);
}

/// 화면 밖 카드는 측정·그리기에서 뺀다. 비용을 메모 수가 아니라 보이는 카드 수에 맞춘다.
/// 단계 전환 중에는 숨기지 않는다. 숨겼다 다시 보이는 카드는 측정을 새로 해 모션 프레임을 밀어낸다.
fn cull(v: &CanvasView) {
    let grow_only = v.session.dive.borrow().is_some();
    cull_for(v, v.session.camera.get(), !grow_only);
}

fn cull_for(v: &CanvasView, camera: Camera, hide: bool) {
    let (width, height) = (
        f64::from(v.widgets.surface.width()),
        f64::from(v.widgets.surface.height()),
    );
    if width < 1. || height < 1. {
        return;
    }
    let margin = CULL_MARGIN / camera.zoom;
    let view = layout::Rect::new(
        camera.x - margin,
        camera.y - margin,
        width / camera.zoom + margin * 2.,
        height / camera.zoom + margin * 2.,
    );
    let shown = v.session.shown.borrow();
    let focused = gtk::prelude::GtkWindowExt::focus(&v.widgets.window);
    for (id, card) in v.widgets.cards.borrow().iter() {
        let on_screen = shown
            .get(id)
            .is_some_and(|p| port::rect_at(*p).intersects(view));
        let keep = on_screen
            || focused
                .as_ref()
                .is_some_and(|w| w == card.root.upcast_ref::<gtk::Widget>());
        if card.root.get_visible() != keep && (keep || hide) {
            card.root.set_visible(keep);
        }
    }
}

pub(in super::super) fn show_card(v: &CanvasView, id: Uuid, position: [f64; 2]) {
    v.session.shown.borrow_mut().insert(id, position);
    if let Some(card) = v.widgets.cards.borrow().get(&id) {
        v.widgets.world.move_(&card.root, position[0], position[1]);
        if !card.root.get_visible() {
            card.root.set_visible(true);
        }
    }
    if v.session.port_owner.get() == Some(id) {
        place_port(v, position);
    }
}

fn place_port(v: &CanvasView, position: [f64; 2]) {
    v.widgets.world.move_(
        &v.widgets.port_handle,
        position[0] + port::CARD_WIDTH - PORT_SIZE / 2.,
        position[1] + port::CARD_HEIGHT / 2. - PORT_SIZE / 2.,
    );
}

/// 호버 중인 카드, 없으면 단일 선택 카드에 연결 핸들을 붙인다.
pub(in super::super) fn update_port(v: &CanvasView) {
    let owner = if v.session.interaction.borrow().hides_port()
        || v.session.camera.get().zoom < PORT_MIN_ZOOM
    {
        None
    } else {
        hover_owner(v).or_else(|| v.session.selection.borrow().single())
    };
    v.session.port_owner.set(owner);
    v.widgets.port_handle.set_visible(owner.is_some());
    if let Some(position) = owner.and_then(|id| v.session.shown.borrow().get(&id).copied()) {
        place_port(v, position);
    }
}

fn hover_owner(v: &CanvasView) -> Option<Uuid> {
    let camera = v.session.camera.get();
    let point = camera.to_world(v.session.pointer.get());
    let margin = PORT_SIZE;
    let shown = v.session.shown.borrow();
    let contains = |position: &[f64; 2]| {
        let rect = port::rect_at(*position);
        point.0 >= rect.x
            && point.0 <= rect.right() + margin
            && point.1 >= rect.y
            && point.1 <= rect.bottom()
    };
    // 대부분은 빈 공간 또는 카드 하나다. 겹칠 때만 GTK의 실제 쌓임 순서를 읽는다.
    let mut hits = shown.iter().filter(|(_, position)| contains(position));
    let (&first, _) = hits.next()?;
    if hits.next().is_none() {
        return Some(first);
    }
    // 위에 그려진 카드를 먼저 찾는다.
    let mut child = v.widgets.world.last_child();
    while let Some(widget) = child {
        child = widget.prev_sibling();
        let Ok(id) = Uuid::parse_str(widget.widget_name().as_str()) else {
            continue;
        };
        if let Some(p) = shown.get(&id) {
            if contains(p) {
                return Some(id);
            }
        }
    }
    None
}

/// 위젯을 다시 만들지 않고 메모별로 조정한다. 호버·포커스·CSS 전환이 끊기지 않는다.
pub(in super::super) fn render(v: &Rc<CanvasView>) {
    let Some(st) = v.state.upgrade() else {
        return;
    };
    // 보던 프로젝트가 지워지면 최상위로 돌아간다. 안의 메모는 저장 계층이 한 단계 위로 올렸다.
    if v.session
        .scope
        .get()
        .is_some_and(|id| !port::exists(&st.borrow(), id))
    {
        v.session.scope.set(None);
    }
    // 지워진 단계의 화면 기억은 버린다. 세션이 길어도 메모 수 이상으로 늘지 않는다.
    v.session
        .cameras
        .borrow_mut()
        .retain(|scope, _| scope.is_none_or(|id| port::exists(&st.borrow(), id)));
    let nodes = port::nodes(
        &st.borrow(),
        &mut v.session.summaries.borrow_mut(),
        v.session.scope.get(),
    );
    let first = !v.session.fitted.get();
    let here: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
    {
        let alive = port::note_ids(&st.borrow());
        let mut cards = v.widgets.cards.borrow_mut();
        let mut parked = v.widgets.parked.borrow_mut();
        let mut shown = v.session.shown.borrow_mut();
        // 지워진 메모의 카드만 버린다. 숨긴 카드는 메모 수 이상으로 늘지 않는다.
        parked.retain(|id, card| {
            let keep = alive.contains(id);
            if !keep {
                v.widgets.world.remove(&card.root);
            }
            keep
        });
        for id in cards.keys().copied().collect::<Vec<_>>() {
            if here.contains(&id) {
                continue;
            }
            let card = cards.remove(&id).expect("listed card");
            shown.remove(&id);
            motion::stop_card(v, id);
            if alive.contains(&id) {
                card.root.set_visible(false);
                card.set_selected(false);
                parked.insert(id, card);
            } else {
                v.widgets.world.remove(&card.root);
            }
        }
    }
    v.session
        .selection
        .borrow_mut()
        .retain(|id| here.contains(&id));
    let mut added = Vec::new();
    for node in &nodes {
        let exists = v.widgets.cards.borrow().contains_key(&node.id);
        if !exists {
            let revived = v.widgets.parked.borrow_mut().remove(&node.id);
            let card = revived.unwrap_or_else(|| {
                let card = Card::new(v, node.id);
                card.root
                    .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
                card
            });
            card.update(node);
            // 첫 화면에서는 카메라가 정해진 뒤 보이는 카드만 측정한다.
            card.root.set_visible(!first);
            v.widgets
                .world
                .move_(&card.root, node.position[0], node.position[1]);
            v.session.shown.borrow_mut().insert(node.id, node.position);
            v.widgets.cards.borrow_mut().insert(node.id, card);
            added.push(node.id);
        } else if let Some(card) = v.widgets.cards.borrow().get(&node.id) {
            card.update(node);
        }
    }
    v.session
        .content
        .set(layout::bounds(nodes.iter().map(|n| n.rect())));
    *v.session.nodes.borrow_mut() = nodes;
    let selection = v.session.selection.borrow().clone();
    for (id, card) in v.widgets.cards.borrow().iter() {
        card.set_selected(selection.contains(*id));
    }
    if v.session.interaction.borrow().is_idle() {
        let targets: Vec<_> = v
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| (n.id, n.position))
            .collect();
        for (id, target) in targets {
            if v.session.shown.borrow().get(&id) != Some(&target) {
                motion::card_to(v, id, target);
            }
        }
    }
    if !first && !added.is_empty() {
        for id in &added {
            enter(v, *id);
        }
        v.session.selection.borrow_mut().set(added.iter().copied());
        v.reveal();
    }
    v.widgets.backdrop.queue_draw();
    super::paint::sync_hud(v);
}

/// 새 카드 등장. 다음 프레임에 시작 클래스를 지워 CSS 전환을 일으킨다.
fn enter(v: &CanvasView, id: Uuid) {
    let cards = v.widgets.cards.borrow();
    let Some(card) = cards.get(&id) else {
        return;
    };
    card.root.add_css_class("entering");
    card.root.add_tick_callback(|widget, _| {
        widget.remove_css_class("entering");
        gtk::glib::ControlFlow::Break
    });
}
