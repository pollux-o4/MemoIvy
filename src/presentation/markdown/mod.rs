mod syntax;
use std::ops::Range;
pub(in crate::presentation::markdown) use syntax::{inline, links, list};

/// 편집기와 카드가 깊이별로 공유하는 목록 기호.
pub(crate) const BULLETS: [&str; 3] = ["•", "◦", "▪"];

mod summary;

pub(crate) use inline::{parse, plain_line};
pub(crate) use links::{link_info, safe_link};
pub(crate) use summary::{fit_preview, summary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Heading1,
    Heading2,
    Bold,
    Italic,
    Strike,
    Link,
    Task,
    TaskDone,
}

#[derive(Debug)]
pub(crate) struct Span {
    pub range: Range<i32>,
    pub style: Style,
}

#[derive(Debug)]
pub(crate) struct Marker {
    pub range: Range<i32>,
    pub owner: Range<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Link {
    /// 눈에 보이는 이름. 숨긴 문법을 뺀 범위다.
    pub range: Range<i32>,
    /// 문법까지 포함한 전체. 커서 줄에서 드러난 원문도 같은 링크로 본다.
    pub owner: Range<i32>,
    /// 원문에서 주소만 있는 범위.
    pub url_range: Range<i32>,
    pub url: String,
}

#[derive(Debug)]
pub(crate) struct Task {
    pub range: Range<i32>,
    pub state: TaskState,
    /// 목록 항목이 소유하는 체크면 그 항목의 단일 원본 인덱스다.
    pub item: Option<usize>,
}

/// 체크 목록 상태. 진행 중은 Obsidian 계열 관례인 `[/]`로 저장한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TaskState {
    Open,
    Doing,
    Done,
}

impl TaskState {
    pub(crate) fn from_marker(marker: &str) -> Option<Self> {
        match marker {
            "[ ]" => Some(Self::Open),
            "[/]" => Some(Self::Doing),
            "[x]" | "[X]" => Some(Self::Done),
            _ => None,
        }
    }

    /// 원문 괄호 안 글자.
    pub(crate) fn mark(self) -> &'static str {
        match self {
            Self::Open => " ",
            Self::Doing => "/",
            Self::Done => "x",
        }
    }

    /// 클릭은 완료를, Shift+클릭은 진행 중을 켜고 끈다.
    pub(crate) fn toggled(self, doing: bool) -> Self {
        match (doing, self) {
            (true, Self::Doing) | (false, Self::Done) => Self::Open,
            (true, _) => Self::Doing,
            (false, _) => Self::Done,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    Bullet,
    Ordered,
    Task,
}

/// 원문 목록 항목. content는 첫 줄 본문부터 들여쓴 이어쓰기 줄 끝까지다.
#[derive(Debug)]
pub(crate) struct ListItem {
    /// 줄 시작부터 본문 직전까지. 들여쓰기·기호·체크 문법과 뒤 공백을 포함한다.
    pub marker: Range<i32>,
    /// 본문부터 줄 끝까지. 비어 있으면 표식을 숨기지 않는다.
    pub content: Range<i32>,
    pub depth: usize,
    pub kind: ListKind,
}

/// 첨부 참조 표기(DATA-003).
pub(crate) const IMAGE_SCHEME: &str = "asset:";
/// 원문에 기록할 수 있는 이미지 너비(px).
pub(crate) const IMAGE_WIDTH: std::ops::RangeInclusive<i32> = 64..=4096;

/// 한 줄 전체가 첨부 이미지 참조인 줄.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Image {
    /// 줄 시작부터 끝(줄바꿈 제외)까지.
    pub line: Range<i32>,
    pub alt: String,
    pub width: Option<i32>,
    /// `asset:<해시>`.
    pub target: String,
}

impl Image {
    /// 너비만 바꾼 원문 한 줄. 이미지 문법의 생성 원본이다.
    pub(crate) fn markup(alt: &str, width: Option<i32>, target: &str) -> String {
        let alt: String = alt
            .chars()
            .filter(|c| !matches!(c, '[' | ']' | '|' | '\n'))
            .collect();
        match width {
            Some(width) => format!("![{alt}|{width}]({target})"),
            None => format!("![{alt}]({target})"),
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct Document {
    pub spans: Vec<Span>,
    pub markers: Vec<Marker>,
    pub links: Vec<Link>,
    pub tasks: Vec<Task>,
    pub items: Vec<ListItem>,
    pub images: Vec<Image>,
}

impl Document {
    /// 줄 순서로 정렬된 항목에서 커서 위치를 찾는다. 줄 끝 커서도 해당 항목에 속한다.
    pub(crate) fn item_at(&self, offset: i32) -> Option<(usize, &ListItem)> {
        let index = self
            .items
            .partition_point(|item| item.marker.start <= offset)
            .checked_sub(1)?;
        let item = &self.items[index];
        (offset <= item.content.end).then_some((index, item))
    }
}

/// GTK offsets count characters; the parser reports UTF-8 byte ranges.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Summary {
    pub title: String,
    pub preview: String,
    /// 제목 줄이 목록 항목이면 그 표식. 카드가 제목 앞에 같이 보여 준다.
    pub title_marker: Option<LineMarker>,
    /// 제목 다음 줄들. 표식을 잃지 않는 구조화된 미리보기다.
    pub lines: Vec<PreviewLine>,
    /// `lines`에 담지 않은 나머지 내용 줄 수.
    pub remaining: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LineMarker {
    Bullet,
    Number(String),
    Task(TaskState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewLine {
    pub marker: Option<LineMarker>,
    pub depth: usize,
    /// 앞선 목록 항목의 표식 폭을 유지해 본문 시작점을 맞춘다.
    pub continuation: bool,
    pub text: String,
}

/// 줄 예산에 맞춘 표시 행. 표식 없는 연속 문단은 한 행으로 합쳐 줄바꿈한다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewRow {
    pub line: PreviewLine,
    pub max_lines: i32,
}

/// 링크 미리보기에 보일 정보. 이름이 아니라 실제 목적지를 드러내는 것이 목적이다.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinkInfo {
    /// 강조할 목적지. 웹은 호스트, 메일은 주소다.
    pub host: String,
    /// 호스트 뒤에 이어지는 경로·쿼리.
    pub rest: String,
    pub mail: bool,
    pub warning: Option<&'static str>,
}

pub(crate) fn task_progress(text: &str) -> (usize, usize) {
    let tasks = parse(text).tasks;
    (
        tasks
            .iter()
            .filter(|task| task.state == TaskState::Done)
            .count(),
        tasks.len(),
    )
}

#[cfg(test)]
mod tests;
