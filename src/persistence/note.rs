const COLORS: [&str; 5] = ["yellow", "pink", "blue", "green", "purple"];
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub text: String,
    pub color: String,
    pub modified: u64,
    #[serde(default)]
    pub canvas: super::canvas::CanvasNode,
}
impl Note {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            text: String::new(),
            color: COLORS[0].into(),
            modified: now(),
            canvas: Default::default(),
        }
    }
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
