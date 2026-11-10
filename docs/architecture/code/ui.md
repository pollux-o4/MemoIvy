# C4 Code · UI 탐색

## 조립·공통 조작

| 관심사 | 탐색 키 |
|---|---|
| 앱·창 조율 | `run`, shell의 `AppState`·`Views`·`Closing`, `wake`·`refresh_all` |
| 삭제 순서 | shell commands의 `delete_notes`·`next_delete` |
| 검색·목록 갱신 | library cards의 `refresh`·`list_key`·`tree`·`location`·`display_title` |
| 생성 선택·모핑 | shared creation의 `configure`, surface의 `Surface` / 실행·허용 콜백은 화면이 공급하고 정책 조합은 shell commands의 `creation_allowed` |
| 저장 이벤트·종료 | shell lifecycle의 `tick`, events의 `apply`, closing의 `settle`·`close_editor`·`close_library` |
| 카드·프로젝트·본문 | library cards의 card / `CardKind`·`card`, grouping / `append`, preview / `build`, disclosure / `button`·`reflect` |
| 공유 조작 | shared의 `confirm_delete`·`close_on_escape`·`find_named`, focus·shortcuts, menus의 rows·dialogs·note |

## 편집기

| 관심사 | 탐색 키 |
|---|---|
| 창 구성·본문 갱신 | editor의 `open_note`, live_editor의 `attach`·`schedule`·`refresh` |
| 표시·입력 상태 | live_editor의 tags / `EditorTags`, layout / `LayoutState`, markers / `MarkerWidgets`, input / `InputState`·`PointerState` |
| 원문 수정 | list_edit의 `handle`·`continue_after_commit`, formatting, `source_slice` |
| 메모 이미지 | live_editor media의 `MediaHost`·`sync_images`·`place_images`, block의 `Block`, import의 `connect_import`·`placement`·`insert_lines`·`move_image`·`drop_target` |
| 링크 카드 | link_preview의 `CardState`·`pointer_left`·`caret_left`; card의 위젯 구성·`fill` |

## 파일 화면·공용 위젯

| 관심사 | 탐색 키 |
|---|---|
| 파일 창 | files의 `FilesView`·`open`·`refresh`, grid의 `Grid`·`drag_source`, detail의 `Detail` |
| 공용 위젯 | widgets의 `show_image`·`ASSET_MIME`·`animations_enabled`, shared의 `confirm` |
| 사용처 갱신 | shell commands의 `reconcile_attachments`·`open_files` |

## 캔버스

| 관심사 | 탐색 키 |
|---|---|
| 화면·세션 · `src/app/canvas/mod.rs` | `CanvasView`·`CanvasWidgets`·`BoardSession`, `selected_views` |
| 규칙·이력 · `src/app/canvas/model` | model의 `Camera`·`Selection`·`History`·`Spring` |
| 표시·입력 · `src/app/canvas` | scene의 board·card·preview, input, panels, motion |
| 저장 연결·요약 캐시 | port의 `NodeView`·`SummaryCache`·`nodes`, 단계 경로·바로가기 `path`·`shortcuts`·`move_targets` |
| 프로젝트 단계·전환 | controller commands의 `go_to`·`switch_scope`·`group_selected`·`move_into`, scene dive의 `capture`·`begin`·`play`·`pose`, panels navigator의 `sync` |
| 보드 그리기 | scene paint의 background / `draw_backdrop`·`draw_grid`, hud / `draw_hud`, edges의 `draw_edges`·curves·edge_tiles / `EdgeTiles`, backdrop / `Backdrop`, board의 `set_camera`·`apply_camera`·`render`(다른 단계 카드 `parked`) |

경계 검사: `architecture_tests`. GTK 시나리오 진입점은 `gtk_tests`, 조건 대기·탐색은 `test_support`; 실제 입력 스모크의 공통 기반은 `memoivy_driver`다. 실행 절차: OPS-003.

- 캔버스 조율: `src/app/canvas/controller`의 window / `open`, commands / `mutate`·`step_history`, camera / `fit_rects`, view / `render`·`sync_selection`.
- 카드 표현: `src/app/canvas/scene/cards`의 card·card_root·preview. geometry의 layout·camera·placement는 GTK 없이 검증한다.
- GTK 시나리오: canvas/tests의 flow·shortcuts·performance, live_editor/tests의 layout·typing·links·editing. 기존 gtk_tests 진입점이 프로세스 격리를 유지한다.
