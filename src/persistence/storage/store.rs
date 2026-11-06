use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

use super::note::{Note, COLORS};

pub struct Store {
    pub notes: Vec<Note>,
}
impl Store {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let notes: Vec<Note> = match fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut ids = std::collections::HashSet::new();
        if !super::links::valid(&notes)
            || notes
                .iter()
                .any(|note: &Note| !ids.insert(note.id) || !COLORS.contains(&note.color.as_str()))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "중복된 메모 ID, 알 수 없는 색상 또는 올바르지 않은 캔버스 연결",
            ));
        }
        Ok(Self { notes })
    }
}
pub fn save(path: &Path, notes: &[Note]) -> io::Result<()> {
    if !super::links::valid(notes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "올바르지 않은 캔버스 데이터",
        ));
    }
    write_atomic(path, |file| {
        serde_json::to_writer(&mut *file, notes)?;
        Ok(())
    })
}

/// DATA-002의 원자 교체. 같은 폴더의 소유자 전용 임시 파일에 쓰고 동기화한 뒤 바꿔 넣는다.
pub(in crate::persistence) fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("저장 폴더가 없습니다"))?;
    fs::create_dir_all(dir)?;
    let temp = dir.join(format!(".write-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        write(&mut file)?;
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
        assert!(store.notes[0].canvas.position.is_none());
        let mut next = Note::new();
        next.canvas.position = Some([64., 64.]);
        store.notes[0].canvas.next.push(next.id);
        store.notes[0].canvas.done = true;
        store.notes.push(next);
        save(&path, &store.notes).unwrap();
        let loaded = Store::load(path.clone()).unwrap();
        assert_eq!(loaded.notes[0].canvas, store.notes[0].canvas);
        let original = fs::read(&path).unwrap();
        store.notes[0].canvas.next.push(Uuid::new_v4());
        assert!(save(&path, &store.notes).is_err());
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
        save(&path, &store.notes).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        save(&path, &store.notes).unwrap();
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
        save(&path, &store.notes).unwrap();
        let mut loaded = Store::load(path.clone()).unwrap();
        assert_eq!(loaded.notes[0].id, note.id);
        assert_eq!(loaded.notes[0].text, note.text);
        assert_eq!(loaded.notes[0].color, "pink");
        loaded.notes.clear();
        save(&path, &loaded.notes).unwrap();
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
