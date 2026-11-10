//! 라이브러리 펼침 상태. 메모 원본과 별도로 원자 저장한다.
use super::store::write_atomic;
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(crate) struct LibraryPreferences {
    path: PathBuf,
    expanded: BTreeSet<Uuid>,
}

impl LibraryPreferences {
    pub fn beside(notes: &Path) -> Self {
        let path = notes.with_file_name("library.json");
        let expanded = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                eprintln!("목록 상태를 읽지 못했습니다: {error}");
                BTreeSet::new()
            }),
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    eprintln!("목록 상태를 읽지 못했습니다: {error}");
                }
                BTreeSet::new()
            }
        };
        Self { path, expanded }
    }

    pub fn expanded(&self) -> &BTreeSet<Uuid> {
        &self.expanded
    }

    pub fn toggle(&mut self, id: Uuid, keep: impl Fn(&Uuid) -> bool) -> io::Result<()> {
        self.expanded.retain(keep);
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
        write_atomic(&self.path, |file| {
            serde_json::to_writer(file, &self.expanded)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_002_expansion_defaults_persists_prunes_and_recovers() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes.json");
        fs::write(&notes, b"original notes").unwrap();
        let id = Uuid::new_v4();
        let gone = Uuid::new_v4();
        let mut prefs = LibraryPreferences::beside(&notes);
        assert!(prefs.expanded().is_empty());
        prefs.toggle(gone, |_| true).unwrap();
        prefs.toggle(id, |other| *other != gone).unwrap();
        let mut prefs = LibraryPreferences::beside(&notes);
        assert_eq!(prefs.expanded(), &BTreeSet::from([id]));
        prefs.toggle(id, |_| true).unwrap();
        assert!(LibraryPreferences::beside(&notes).expanded().is_empty());
        fs::write(dir.path().join("library.json"), b"broken").unwrap();
        let mut prefs = LibraryPreferences::beside(&notes);
        assert!(prefs.expanded().is_empty());
        fs::remove_file(dir.path().join("library.json")).unwrap();
        fs::create_dir(dir.path().join("library.json")).unwrap();
        assert!(prefs.toggle(id, |_| true).is_err());
        assert!(prefs.expanded().contains(&id));
        assert_eq!(fs::read(&notes).unwrap(), b"original notes");
    }
}
