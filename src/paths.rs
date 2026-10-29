use std::path::PathBuf;

pub(crate) fn data_path() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| PathBuf::from(v).is_absolute())
    {
        return Ok(PathBuf::from(dir).join("linux-sticky-memo/notes.json"));
    }
    std::env::var_os("HOME")
        .filter(|v| PathBuf::from(v).is_absolute())
        .map(|v| PathBuf::from(v).join(".local/share/linux-sticky-memo/notes.json"))
        .ok_or_else(|| "HOME 환경 변수가 없어 저장 위치를 찾을 수 없습니다.".into())
}
