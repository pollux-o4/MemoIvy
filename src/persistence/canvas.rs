use serde::{Deserialize, Serialize};
use uuid::Uuid;

// shortcut: 로컬 단일 보드 좌표를 제한해 잘못된 데이터와 과도한 영역을 방지, 다중 프로젝트 또는 대형 보드 요구 시 공간 모델 재검토.
pub const LIMIT: f64 = 20_000.;
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct CanvasNode {
    pub position: Option<[f64; 2]>,
    pub next: Vec<Uuid>,
    pub done: bool,
}
