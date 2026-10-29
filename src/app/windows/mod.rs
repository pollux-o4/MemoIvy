mod editor;
pub(in crate::app) mod focus;
mod library;
pub(in crate::app) mod shortcuts;

pub(super) use editor::open_note;
pub(super) use library::build;
