//! 메모 캔버스 프런트엔드 모듈. 화면 API는 `open`·`refresh`·`update_status`·`CanvasView::destroy`다.
//! 초기 배치·빈자리 계산은 shell 명령에서 사용하는 보조 진입점이다.
//!
//! - `model`: GTK 없는 규칙 (카메라·정렬·선택·스프링)
//! - `scene`: 보드 위 그림과 카드 위젯
//! - `input`: 포인터·키보드 해석
//! - `panels`: 떠 있는 도구 (독·정렬 막대·선택 패널)
//! - `motion`: 프레임 구동 / `port`: 저장 계층과의 유일한 경계
mod controller;
pub(in crate::app) use controller::{open, refresh, update_status};
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
    backdrop: DrawingArea,
    hud: DrawingArea,
    panel: Revealer,
    inspector: GtkBox,
    connect_menu: RefCell<Option<gtk::MenuButton>>,
    align_bar: Revealer,
    align_count: Label,
    distribute: [Button; 2],
    empty: GtkBox,
    status: Label,
    summary: Label,
    retry: Button,
    dock: GtkBox,
    undo: Button,
    redo: Button,
    zoom_label: Button,
    cards: RefCell<HashMap<Uuid, scene::card::Card>>,
}

struct BoardSession {
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
}

pub(in crate::app) fn vacant_position(notebook: &port::Notebook) -> [f64; 2] {
    port::vacant(notebook)
}

pub(in crate::app) fn initialize_positions(notebook: &mut port::Notebook) {
    port::initialize_positions(notebook);
}
