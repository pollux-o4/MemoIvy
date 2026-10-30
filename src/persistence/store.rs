use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const COLORS: [(&str, &str); 5] = [
    ("yellow", "노랑"),
    ("pink", "분홍"),
    ("blue", "파랑"),
    ("green", "초록"),
    ("purple", "보라"),
];

pub use super::note::{now, Note};

pub struct Store {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub notes: Vec<Note>,
}
impl Store {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let mut notes: Vec<Note> = match fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut ids = std::collections::HashSet::new();
        if !super::canvas::valid(&notes)
            || notes.iter().any(|note: &Note| {
                !ids.insert(note.id) || !COLORS.iter().any(|(name, _)| *name == note.color)
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "중복된 메모 ID, 알 수 없는 색상 또는 올바르지 않은 캔버스 연결",
            ));
        }
        for (i, note) in notes.iter_mut().enumerate() {
            if note.canvas.position.is_none() {
                note.canvas.position = Some(super::canvas::position(note, i));
            }
        }
        Ok(Self { path, notes })
    }
    #[allow(dead_code)]
    pub fn save(&self) -> io::Result<()> {
        save(&self.path, &self.notes)
    }
}
pub fn save(path: &Path, notes: &[Note]) -> io::Result<()> {
    if !super::canvas::valid(notes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "올바르지 않은 캔버스 데이터",
        ));
    }
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("저장 폴더가 없습니다"))?;
    fs::create_dir_all(dir)?;
    let temp = dir.join(format!(".notes-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        serde_json::to_writer(&mut file, notes)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_001_legacy_and_graph_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let id = Uuid::new_v4();
        fs::write(
            &path,
            format!(r#"[{{"id":"{id}","text":"old","color":"yellow","modified":1}}]"#),
        )
        .unwrap();
        let mut store = Store::load(path.clone()).unwrap();
        assert_eq!(store.notes[0].id, id);
        assert!(store.notes[0].canvas.position.is_some());
        let mut next = Note::new();
        next.canvas.position = Some(super::super::canvas::vacant(&store.notes));
        store.notes[0].canvas.next.push(next.id);
        store.notes[0].canvas.done = true;
        store.notes.push(next);
        store.save().unwrap();
        let loaded = Store::load(path.clone()).unwrap();
        assert_eq!(loaded.notes[0].canvas, store.notes[0].canvas);
        let original = fs::read(&path).unwrap();
        store.notes[0].canvas.next.push(Uuid::new_v4());
        assert!(store.save().is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(&path, serde_json::to_vec(&store.notes).unwrap()).unwrap();
        assert!(Store::load(path).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn data_002_saved_files_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let store = Store::load(path.clone()).unwrap();
        store.save().unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        store.save().unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    }
    #[test]
    fn data_001_round_trip_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut store = Store::load(path.clone()).unwrap();
        let mut note = Note::new();
        note.text = "안녕하세요 🌻\n두 번째 줄".into();
        note.color = "pink".into();
        store.notes.push(note.clone());
        store.save().unwrap();
        let mut loaded = Store::load(path.clone()).unwrap();
        assert_eq!(loaded.notes[0].id, note.id);
        assert_eq!(loaded.notes[0].text, note.text);
        assert_eq!(loaded.notes[0].color, "pink");
        loaded.notes.clear();
        loaded.save().unwrap();
        assert!(Store::load(path).unwrap().notes.is_empty());
    }
    #[test]
    fn data_001_corrupt_data_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        fs::write(&path, b"{broken").unwrap();
        assert!(Store::load(path.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"{broken");
    }
    #[test]
    fn data_001_invalid_ids_and_colors_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let note = Note::new();
        let duplicate = serde_json::to_vec(&[note.clone(), note.clone()]).unwrap();
        fs::write(&path, &duplicate).unwrap();
        assert!(Store::load(path.clone()).is_err());
        assert_eq!(fs::read(&path).unwrap(), duplicate);
        let mut invalid = note;
        invalid.color = "unknown".into();
        let invalid = serde_json::to_vec(&[invalid]).unwrap();
        fs::write(&path, &invalid).unwrap();
        assert!(Store::load(path.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), invalid);
    }
    #[test]
    fn data_002_failed_save_keeps_existing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        fs::create_dir(&path).unwrap();
        assert!(save(&path, &[Note::new()]).is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
