//! 메모 캔버스 프런트엔드 모듈. 화면 API는 `open`·`refresh`·`update_status`·`CanvasView::destroy`다.
//! 초기 배치·빈자리 계산, 지금 보는 프로젝트(`scope_for`), 묶기 완료(`adopt`)는 shell 명령의 보조 진입점이다.
//! 캔버스는 한 번에 한 단계(scope)를 보여 준다. 자식이 있는 메모는 들어갈 수 있는 프로젝트 카드가 된다.
//!
//! - `model`: GTK 없는 규칙 (카메라·정렬·선택·스프링)
//! - `scene`: 보드 위 그림과 카드 위젯
//! - `input`: 포인터·키보드 해석
//! - `panels`: 떠 있는 도구 (독·정렬 막대·선택 패널)
//! - `motion`: 프레임 구동 / `port`: 저장 계층과의 유일한 경계
mod controller;
pub(in crate::app) use controller::{adopt, open, refresh, scope_for, show_project, update_status};
mod input;
mod model;
mod motion;
mod panels;
mod port;
mod scene;
#[cfg(test)]
pub(in crate::app) mod tests;

use crate::app::{
    shared::header,
    shell::{
        commands as shell_commands, lifecycle,
        state::{AppState, State},
    },
};
use crate::presentation::icons;
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Fixed, Label,
    Orientation, Overlay, Revealer,
};
use input::Interaction;
use model::{
    camera::Camera,
    layout::{self, Guide, Moves, Rect},
    selection::Selection,
};
use port::NodeView;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};
use uuid::Uuid;

const HISTORY_LIMIT: usize = 40;
const FIT_PADDING: f64 = 72.;
const MIN_BOARD: (i32, i32) = (560, 360);

pub(in crate::app) struct CanvasView {
    state: Weak<RefCell<AppState>>,
    app: Application,
    widgets: CanvasWidgets,
    session: BoardSession,
    motion: RefCell<motion::MotionState>,
    history: RefCell<model::history::History<port::Snapshot>>,
}

struct CanvasWidgets {
    window: ApplicationWindow,
    surface: Overlay,
    stage: Fixed,
    world: Fixed,
    port_handle: GtkBox,
    backdrop: scene::paint::backdrop::Backdrop,
    hud: DrawingArea,
    /// 단계 전환 때 떠나는 화면을 굳힌 텍스처를 그린다. 전환 밖에서는 숨겨 비용이 없다.
    freeze: scene::paint::backdrop::Backdrop,
    panel: Revealer,
    inspector: GtkBox,
    connect_menu: RefCell<Option<gtk::MenuButton>>,
    align_bar: Revealer,
    align_count: Label,
    distribute: [Button; 2],
    empty: GtkBox,
    empty_title: Label,
    empty_sub: Label,
    status: Label,
    summary: Label,
    retry: Button,
    dock: GtkBox,
    undo: Button,
    redo: Button,
    zoom_label: Button,
    navigator: panels::Navigator,
    project_actions: [Button; 2],
    cards: RefCell<HashMap<Uuid, scene::card::Card>>,
    /// 다른 단계에 있는 메모의 숨긴 카드. 단계를 오갈 때 위젯을 새로 만들지 않고 되살린다.
    parked: RefCell<HashMap<Uuid, scene::card::Card>>,
}

struct BoardSession {
    /// 지금 보는 프로젝트. None은 최상위 보드다.
    scope: Cell<Option<Uuid>>,
    /// 단계마다 마지막으로 본 화면. 돌아오면 그 자리에서 이어 본다.
    cameras: RefCell<HashMap<Option<Uuid>, Camera>>,
    /// 들어가기·나오기 전환 중. 그사이 입력이 이전 단계를 바꾸지 않게 한다.
    diving: Cell<bool>,
    dive: RefCell<Option<scene::dive::Dive>>,
    navigator_key: RefCell<String>,
    inspector_key: RefCell<String>,
    nodes: RefCell<Vec<NodeView>>,
    content: Cell<Option<Rect>>,
    shown: RefCell<HashMap<Uuid, [f64; 2]>>,
    selection: RefCell<Selection>,
    camera: Cell<Camera>,
    camera_queued: Cell<bool>,
    scroll_panning: Cell<bool>,
    fitted: Cell<bool>,
    interaction: RefCell<Interaction>,
    guides: RefCell<Vec<Guide>>,
    pointer: Cell<(f64, f64)>,
    space: Cell<bool>,
    port_owner: Cell<Option<Uuid>>,
    last_click: Cell<Option<(Uuid, u32)>>,
    drag_modifiers: Cell<(bool, bool)>,
    drag_styled: Cell<bool>,
    presenting: Cell<bool>,
    summaries: RefCell<port::SummaryCache>,
    edge_tiles: RefCell<scene::paint::edge_tiles::EdgeTiles>,
}

pub(in crate::app) fn vacant_position(notebook: &port::Notebook, scope: Option<Uuid>) -> [f64; 2] {
    port::vacant(notebook, scope)
}

pub(in crate::app) fn initialize_positions(notebook: &mut port::Notebook) {
    port::initialize_positions(notebook);
}
