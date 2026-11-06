//! assets 폴더의 파일 I/O. 바이트 서명으로 형식을 판별하고 SHA-256 이름으로 한 번만 쓴다.
use super::super::store::write_atomic;
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// 한 파일 상한. 스크린샷·사진에 충분하고 실수로 넣은 대용량 파일은 막는다.
pub const MAX_BYTES: u64 = 20 * 1024 * 1024;
const FORMATS: [&str; 4] = ["png", "jpg", "gif", "webp"];

/// SHA-256 소문자 16진수 64자. 본문에는 `asset:<id>`로 쓴다.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(String);

impl AssetId {
    pub fn parse(text: &str) -> Option<Self> {
        (text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then(|| Self(text.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// 본문 참조 대상. DATA-003의 `asset:` 표기다.
    pub fn target(&self) -> String {
        format!("{}{}", super::SCHEME, self.0)
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 사용자에게 그대로 보여 줄 수 있는 거부 이유.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportError(pub(super) String);

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ImportError {}

impl From<&str> for ImportError {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

fn format_of(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// 첨부 파일 저장소. 폴더 경로만 들고 있어 편집 창마다 복제해 쓴다.
#[derive(Clone, Debug)]
pub struct AssetStore {
    dir: PathBuf,
}

impl AssetStore {
    pub(super) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// 바이트를 복사해 저장한다. 이미 있으면 다시 쓰지 않는다.
    pub fn import(&self, bytes: &[u8]) -> Result<AssetId, ImportError> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err(ImportError("20MB보다 큰 파일은 넣을 수 없어요".into()));
        }
        let format = format_of(bytes)
            .ok_or_else(|| ImportError("PNG·JPEG·GIF·WebP 이미지만 넣을 수 있어요".into()))?;
        let id = AssetId(
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        );
        let path = self.dir.join(format!("{id}.{format}"));
        if !path.exists() {
            write_atomic(&path, |file| file.write_all(bytes))
                .map_err(|e| ImportError(format!("이미지를 저장하지 못했어요: {e}")))?;
        }
        Ok(id)
    }

    /// 외부 파일을 복사해 넣는다. 원본 경로는 기억하지 않는다.
    pub fn import_file(&self, path: &Path) -> Result<AssetId, ImportError> {
        let unreadable = |e: io::Error| ImportError(format!("파일을 읽지 못했어요: {e}"));
        let file = fs::File::open(path).map_err(unreadable)?;
        if file.metadata().map_err(unreadable)?.len() > MAX_BYTES {
            return Err(ImportError("20MB보다 큰 파일은 넣을 수 없어요".into()));
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(unreadable)?;
        self.import(&bytes)
    }

    pub fn path(&self, id: &AssetId) -> Option<PathBuf> {
        FORMATS
            .iter()
            .map(|format| self.dir.join(format!("{id}.{format}")))
            .find(|path| path.is_file())
    }

    /// 폴더의 첨부 파일과 크기. 이름이 규칙에 맞지 않는 파일은 건드리지 않는다.
    pub(super) fn list(&self) -> Vec<(AssetId, u64)> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let format = path.extension()?.to_str()?;
                let id = AssetId::parse(path.file_stem()?.to_str()?)?;
                let meta = entry.metadata().ok().filter(|m| m.is_file())?;
                FORMATS.contains(&format).then_some((id, meta.len()))
            })
            .collect()
    }

    pub(super) fn remove(&self, id: &AssetId) -> io::Result<()> {
        while let Some(path) = self.path(id) {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
