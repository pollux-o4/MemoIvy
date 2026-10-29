use serde::{Deserialize, Serialize};
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub const COLORS: [(&str, &str); 5] = [
    ("yellow", "노랑"),
    ("pink", "분홍"),
    ("blue", "파랑"),
    ("green", "초록"),
    ("purple", "보라"),
];

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub text: String,
    pub color: String,
    pub modified: u64,
}
impl Note {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            text: String::new(),
            color: COLORS[0].0.into(),
            modified: now(),
        }
    }
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct Store {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub notes: Vec<Note>,
}
impl Store {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let notes = match fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut ids = std::collections::HashSet::new();
        if notes.iter().any(|note: &Note| {
            !ids.insert(note.id) || !COLORS.iter().any(|(name, _)| *name == note.color)
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "중복된 메모 ID 또는 알 수 없는 색상",
            ));
        }
        Ok(Self { path, notes })
    }
    #[allow(dead_code)]
    pub fn save(&self) -> io::Result<()> {
        save(&self.path, &self.notes)
    }
}
pub fn save(path: &Path, notes: &[Note]) -> io::Result<()> {
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
