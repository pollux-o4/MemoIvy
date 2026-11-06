use std::{fs, path::PathBuf};

pub(crate) fn data_path() -> Result<PathBuf, String> {
    let root = if let Some(dir) =
        std::env::var_os("XDG_DATA_HOME").filter(|v| PathBuf::from(v).is_absolute())
    {
        PathBuf::from(dir)
    } else {
        std::env::var_os("HOME")
            .filter(|v| PathBuf::from(v).is_absolute())
            .map(|v| PathBuf::from(v).join(".local/share"))
            .ok_or_else(|| "HOME 환경 변수가 없어 저장 위치를 찾을 수 없습니다.".to_owned())?
    };

    let current = root.join("memoivy");
    // Migration-only compatibility path for notes and attachments saved by older releases.
    let legacy = root.join("linux-sticky-memo");
    // shortcut: Snap data roots cannot see the native legacy folder by default; add an explicit import flow after reviewing Snap permissions.
    Ok(resolve_data_dir(&legacy, &current).join("notes.json"))
}

fn resolve_data_dir(legacy: &std::path::Path, current: &std::path::Path) -> PathBuf {
    let current_notes = current.join("notes.json");
    let legacy_notes = legacy.join("notes.json");

    if current_notes.is_file() {
        // If both directories have notes, prefer MemoIvy and leave the legacy copy untouched.
        return current.to_owned();
    }
    if legacy_notes.is_file() {
        if current.exists() {
            // Keep both copies intact; continue using the only directory with notes.
            return legacy.to_owned();
        }
        if legacy.is_dir() && fs::rename(legacy, current).is_ok() {
            return current.to_owned();
        }
        if current_notes.is_file() {
            // Another process may have completed the atomic move after our initial check.
            return current.to_owned();
        }
        return legacy.to_owned();
    }

    if legacy.is_dir() && !current.exists() && fs::rename(legacy, current).is_ok() {
        return current.to_owned();
    }
    current.to_owned()
}
