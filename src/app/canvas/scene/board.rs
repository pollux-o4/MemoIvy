use super::super::{
    model::{camera::Camera, layout},
    motion, port, CanvasView,
};
use super::card::{Card, PORT_SIZE};
use gtk::prelude::*;
use std::rc::Rc;
use uuid::Uuid;

const FAR_ZOOM: f64 = 0.45;
const TINY_ZOOM: f64 = 0.2;
const CULL_MARGIN: f64 = 48.;
const PORT_MIN_ZOOM: f64 = 0.3;

/// 월드 전체에 변환 하나만 적용해 확대·이동 비용을 카드 수와 무관하게 유지한다.
pub(in super::super) fn set_camera(v: &CanvasView, camera: Camera) {
    let camera = v.keep_camera(camera);
    v.session.camera.set(camera);
    let transform = gtk::gsk::Transform::new()
        .translate(&gtk::graphene::Point::new(
            (-camera.x * camera.zoom).round() as f32,
            (-camera.y * camera.zoom).round() as f32,
        ))
        .scale(camera.zoom as f32, camera.zoom as f32);
    v.widgets
        .stage
        .set_child_transform(&v.widgets.world, Some(&transform));
    v.widgets
        .zoom_label
        .set_label(&format!("{:.0}%", camera.zoom * 100.));
    for (class, limit) in [("far", FAR_ZOOM), ("tiny", TINY_ZOOM)] {
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
fn cull(v: &CanvasView) {
    let (width, height) = (
        f64::from(v.widgets.surface.width()),
        f64::from(v.widgets.surface.height()),
    );
    if width < 1. || height < 1. {
        return;
    }
    let camera = v.session.camera.get();
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
        if card.root.get_visible() != keep {
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
    // 위에 그려진 카드를 먼저 찾는다.
    let mut child = v.widgets.world.last_child();
    while let Some(widget) = child {
        child = widget.prev_sibling();
        let Ok(id) = Uuid::parse_str(widget.widget_name().as_str()) else {
            continue;
        };
        if let Some(p) = shown.get(&id) {
            let r = port::rect_at(*p);
            if point.0 >= r.x
                && point.0 <= r.right() + margin
                && point.1 >= r.y
                && point.1 <= r.bottom()
            {
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
    let nodes = port::nodes(&st.borrow(), &mut v.session.summaries.borrow_mut());
    let first = !v.session.fitted.get();
    {
        let mut cards = v.widgets.cards.borrow_mut();
        let mut shown = v.session.shown.borrow_mut();
        cards.retain(|id, card| {
            let keep = nodes.iter().any(|n| n.id == *id);
            if !keep {
                v.widgets.world.remove(&card.root);
                shown.remove(id);
                motion::stop_card(v, *id);
            }
            keep
        });
    }
    v.session
        .selection
        .borrow_mut()
        .retain(|id| nodes.iter().any(|n| n.id == id));
    let mut added = Vec::new();
    for node in &nodes {
        let exists = v.widgets.cards.borrow().contains_key(&node.id);
        if !exists {
            let card = Card::new(v, node.id);
            card.update(node);
            card.root
                .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
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
