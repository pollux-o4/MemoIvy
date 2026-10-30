# Code · UI 탐색

- 앱·저장 조율: `run`, `AppState`, `EditorWindow`, `Operation`, controller의 `mark`·`dispatch`·`tick`.
- 창·내용: library `build`, editor `open_note`, cards `refresh`, menus `menu_for`, live_editor `attach`, formatting `handle`, markdown `parse`.
- 캔버스: `CanvasView` 조립·`mutate`·되돌리기, model `Camera`·`align`·`snap`·`Spring`, scene board `render`·`set_camera`, card `Card`, paint, input `Interaction`·`follow`·`handle`, panels `sync`, motion `camera_to`·`card_to`·`advance`, port `nodes`.
- 공유 기반: chrome `header`, menus `menu_popover`·`menu_row`, focus `watch`, shortcuts `attach`, theme `is_dark`, presentation `install`, styles 토큰.

관심사로 파일을 좁힌 뒤 대표 심볼을 검색한다. 검증: CANVAS-001, CANVAS-002, CANVAS-003, ADR-0010 경계 검사, FMT-001, FMT-002, FOCUS-001, KEY-001.
