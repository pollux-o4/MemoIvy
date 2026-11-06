//! 캔버스의 순수 규칙. GTK·저장소를 모르므로 화면 없이 검증한다.
mod geometry;
pub(in crate::app::canvas) use geometry::{camera, layout, placement};
pub(super) mod history;
pub(super) mod selection;
pub(super) mod spring;
