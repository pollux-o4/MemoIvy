# Code · 저장 탐색

- 공개 계약: persistence 루트의 `Notebook`, `Note`, `CanvasEdit`, `SaveStatus`, `NotebookEvent`, `LoadError`, `SaveError`.
- 변경·저장 정책: notebook의 `set_text`, `set_color`, `edit_canvas`, `request_create`, `request_delete`, `flush`, `tick`, `needs_tick`.
- 내부 구현: note(불변식·직렬화), links(그래프), store(로드·원자 저장), save_queue(단일 writer·revision), paths(저장 위치).
- 화면 배치·색상 문구는 프런트 소유다. 옛 메모의 누락 위치는 시작 시 한 번 채우고 저장한다.
- 검증: SAVE-001, SAVE-002, DATA-001, DATA-002, CANVAS-001, ADR-0010.
