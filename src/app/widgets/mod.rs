//! 기능 화면이 함께 쓰는 위젯. 상태·저장을 모르고 받은 값만 그린다.
mod viewer;

pub(in crate::app) use viewer::show_image;

/// 파일 화면에서 편집 창으로 끄는 첨부의 형식. 값은 해시 64자다.
pub(in crate::app) const ASSET_MIME: &str = "application/x-memoivy-asset";

/// 모션 줄이기 설정. 꺼져 있으면 전환 없이 바로 바꾼다.
pub(in crate::app) fn animations_enabled() -> bool {
    gtk::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations())
}
