# C4 · UI 코드 라우팅

[상위: UI 컴포넌트](../03-ui-components.md) · [저장 코드](persistence.md) · [기반 코드](platform.md)

수동 클래스 그림 대신 실제 함수와 타입으로 이동한다. Rust 소스 링크는 저장소 상대 경로이며 함수명은 탐색 키다.

| 관심사 | 소스 | 시작 지점 |
|---|---|---|
| 실행 진입 | [main.rs](../../src/main.rs) | `main` → `app::run` |
| 앱 조립·공유 상태 | [app/mod.rs](../../src/app/mod.rs) | `run`, `AppState`, `Operation`, `EditorWindow` |
| 목록 창 생성·검색·새 메모 | [library.rs](../../src/app/library.rs) | `build` |
| 필터링·정렬·카드 표시 | [cards.rs](../../src/app/cards.rs) | `refresh` |
| 편집·색상·재시도·닫기 intent | [editor.rs](../../src/app/editor.rs) | `open_note` |
| 삭제 확인·삭제 intent | [menus.rs](../../src/app/menus.rs) | `menu_for` |
| 창 버튼·titlebar·확대/복원 | [chrome.rs](../../src/app/chrome.rs) | `header` |

창을 찾는 기준, 공유 상태 타입, 위젯 간 연결은 `app/mod.rs`와 각 생성 함수에서 확인한다. 목록의 시각적 표현과 저장 결과 조율을 같은 파일에서 찾지 않아도 되도록 분리되어 있다. 저장·닫기 진행 흐름은 다음 [저장 코드 라우팅](persistence.md)에서 이어진다.
