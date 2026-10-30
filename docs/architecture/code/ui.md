# Code · UI 탐색

- 조립·조율: `run`, shell의 `AppState`, `Views`, `LibraryView`, `Closing`, commands, lifecycle의 `wake`·`tick`, views의 `refresh_all`.
- 기능: library의 `build`·cards, editor의 `open_note`·live_editor·formatting, canvas의 `CanvasView`·port.
- 캔버스 내부: `CanvasWidgets`, `BoardSession`, `History`, model의 `Camera`·`align`·`snap`·`Spring`, scene, input, panels, motion.
- 공유 기반: shared의 chrome·menus·focus·shortcuts, presentation의 markdown·theme·icons·styles.

기능 간 창 열기·갱신은 shell로 보낸다. 저장 변경은 Notebook의 공개 계약만 사용한다. 검증: ADR-0010, CANVAS-001, CANVAS-002, CANVAS-003, CANVAS-004, FMT-001, FMT-003, FOCUS-001, KEY-001.
