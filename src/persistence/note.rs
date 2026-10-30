pub(super) const COLORS: [&str; 5] = ["yellow", "pink", "blue", "green", "purple"];
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub(super) id: Uuid,
    pub(super) text: String,
    pub(super) color: String,
    pub(super) modified: u64,
    #[serde(default)]
    pub(super) canvas: super::canvas::CanvasNode,
}
impl Note {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn color(&self) -> &str {
        &self.color
    }
    pub fn modified(&self) -> u64 {
        self.modified
    }
    pub fn canvas(&self) -> &super::canvas::CanvasNode {
        &self.canvas
    }

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
