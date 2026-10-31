//! 메모·불변식·저장 정책의 공개 경계. 파일 형식과 writer는 내부 구현이다.
//! Note는 읽기 전용이며 변경은 Notebook/CanvasEdit을 통해 저장까지 예약한다.
//! 호출자는 같은 스레드에서 tick을 구동하고 needs_tick이 거짓이면 폴링을 멈춘다.
mod storage;
use storage::{paths, save_queue, store};
mod domain;
use domain::{canvas, links, note};
mod notebook;

pub(crate) use canvas::{CanvasNode, LIMIT};
pub(crate) use note::Note;
pub(crate) use notebook::{CanvasEdit, Notebook, NotebookEvent, SaveStatus};
pub(crate) use notebook::{LoadError, SaveError};
pub(crate) use paths::data_path;

#[cfg(test)]
mod contract_tests;
