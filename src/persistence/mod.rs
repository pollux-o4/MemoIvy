mod canvas;
mod links;
mod note;
mod notebook;
mod paths;
mod save_queue;
mod store;

pub(crate) use canvas::{CanvasNode, LIMIT};
pub(crate) use note::Note;
pub(crate) use notebook::{CanvasEdit, Notebook, NotebookEvent, SaveStatus};
pub(crate) use notebook::{LoadError, SaveError};
pub(crate) use paths::data_path;

#[cfg(test)]
mod contract_tests {
    use super::{Notebook, NotebookEvent, SaveStatus};
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    fn settle(notebook: &mut Notebook) {
        let deadline = Instant::now() + Duration::from_secs(5);
        notebook.flush();
        while notebook.needs_tick() {
            for event in notebook.tick(Instant::now()) {
                if let NotebookEvent::SaveFailed(error) | NotebookEvent::OperationFailed(error) =
                    event
                {
                    panic!("{error}");
                }
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(notebook.status(), SaveStatus::Saved);
    }

    #[test]
    fn data_001_public_boundary_preserves_initialized_legacy_positions_after_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        std::fs::write(&path, format!(r#"[{{"id":"{a}","text":"old a","color":"yellow","modified":1}},{{"id":"{b}","text":"old b","color":"pink","modified":2}}]"#)).unwrap();
        let mut notebook = Notebook::open(path.clone()).unwrap();
        assert!(notebook
            .notes()
            .iter()
            .all(|note| note.canvas().position.is_none()));
        assert!(notebook.edit_canvas(|edit| {
            edit.set_position(a, [64., 64.]);
            edit.set_position(b, [384., 64.]);
        }));
        settle(&mut notebook);
        assert!(notebook.request_delete(a));
        settle(&mut notebook);
        let notebook = Notebook::open(path).unwrap();
        assert_eq!(notebook.notes().len(), 1);
        assert_eq!(
            notebook.note(b).unwrap().canvas().position,
            Some([384., 64.])
        );
        assert_eq!(notebook.note(b).unwrap().text(), "old b");
    }
}
