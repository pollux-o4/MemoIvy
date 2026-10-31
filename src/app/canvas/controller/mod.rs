//! 캔버스 조율: 창 수명, 변경 명령, 카메라, 화면 동기화.
use super::*;
mod camera;
mod commands;
mod view;
mod window;
pub(in crate::app) use window::{open, refresh, update_status};
