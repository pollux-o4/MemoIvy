# C4 Code · UI 탐색

## 조립·공통 조작

| 관심사 | 탐색 키 |
|---|---|
| 앱·창 조율 | `run`, shell의 `AppState`·`Views`·`Closing`, `wake`·`refresh_all` |
| 삭제 순서 | shell commands의 `delete_notes`·`next_delete` |
| 검색·목록 갱신 | library cards의 `refresh`·`list_key` |
| 공유 조작 | shared의 `confirm_delete`·`close_on_escape`·`find_named`, focus·shortcuts |

## 편집기

| 관심사 | 탐색 키 |
|---|---|
| 창 구성·본문 갱신 | editor의 `open_note`, live_editor의 `attach`·`schedule`·`refresh` |
| 표시·입력 상태 | live_editor의 tags / `EditorTags`, layout / `LayoutState`, markers / `MarkerWidgets`, input / `InputState`·`PointerState` |
| 원문 수정 | list_edit의 `handle`·`continue_after_commit`, formatting, `source_slice` |
| 링크 카드 | link_preview의 `CardState`·`pointer_left`·`caret_left`; card의 위젯 구성·`fill` |

## 캔버스

| 관심사 | 탐색 키 |
|---|---|
| 화면·세션 · `src/app/canvas/mod.rs` | `CanvasView`·`CanvasWidgets`·`BoardSession`, `selected_views` |
| 규칙·이력 · `src/app/canvas/model` | model의 `Camera`·`Selection`·`History`·`Spring` |
| 표시·입력 · `src/app/canvas` | scene의 board·card·preview, input, panels, motion |
| 저장 연결·요약 캐시 | port의 `NodeView`·`SummaryCache`·`nodes` |

경계 검사: `architecture_tests`. GTK 시나리오 진입점은 `gtk_tests`, 조건 대기·탐색은 `test_support`; 실제 입력 스모크의 공통 기반은 `lsm_driver`다. 실행 절차: OPS-003.

- 캔버스 조율: `src/app/canvas/controller`의 window / `open`, commands / `mutate`·`step_history`, camera / `fit_rects`, view / `render`·`sync_selection`.
- 카드 표현: `src/app/canvas/scene/cards`의 card·card_root·preview. geometry의 layout·camera·placement는 GTK 없이 검증한다.
- GTK 시나리오: canvas/tests의 flow·shortcuts·performance, live_editor/tests의 layout·typing·links·editing. 기존 gtk_tests 진입점이 프로세스 격리를 유지한다.
