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
| 화면·세션 | `CanvasView`·`CanvasWidgets`·`BoardSession`, `selected_views` |
| 규칙·이력 | model의 `Camera`·`Selection`·`History`·`Spring` |
| 표시·입력 | scene의 board·card·preview, input, panels, motion |
| 저장 연결·요약 캐시 | port의 `NodeView`·`SummaryCache`·`nodes` |

경계 검사: `architecture_tests`. GTK 시나리오 진입점은 `gtk_tests`, 조건 대기·탐색은 `test_support`; 실제 입력 스모크의 공통 기반은 `lsm_driver`다. 실행 절차: OPS-003.
