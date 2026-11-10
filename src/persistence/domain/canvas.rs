use serde::{Deserialize, Serialize};
use uuid::Uuid;

// shortcut: 각 프로젝트 보드의 좌표를 20,000px로 제한, 더 큰 보드 요구가 생기면 공간 모델 재검토.
pub const LIMIT: f64 = 20_000.;
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct CanvasNode {
    pub position: Option<[f64; 2]>,
    pub next: Vec<Uuid>,
    pub done: bool,
    /// 비어 있어도 프로젝트 정체성을 유지한다. 기존 포함 관계는 읽을 때 승격한다.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub project: bool,
    /// 이 메모를 담은 프로젝트 메모. 없으면 최상위다.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Uuid>,
    /// 사용자가 바로가기에 고정한 프로젝트.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}

pub(in crate::persistence) fn valid_position(p: [f64; 2]) -> bool {
    p.iter().all(|v| v.is_finite() && (0. ..=LIMIT).contains(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_002_position_limits_cover_nonfinite_and_edges() {
        assert!(valid_position([0., LIMIT]));
        for position in [
            [-1., 0.],
            [LIMIT + 1., 0.],
            [f64::NAN, 0.],
            [0., f64::INFINITY],
        ] {
            assert!(!valid_position(position));
        }
    }
}
