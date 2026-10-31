//! 보드에 보이는 것: 카드 위젯 조정(board·card)과 벡터 그림(paint).
mod cards;
pub(in crate::app::canvas) use cards::{card, card_root, preview};
pub(super) mod board;
pub(super) mod paint;
