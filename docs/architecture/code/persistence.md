# C4 Code · 저장 탐색

| 관심사 | 탐색 키 |
|---|---|
| 공개 경계 | persistence 루트의 `Notebook`·`Note`·`CanvasEdit`, `SaveStatus`·`NotebookEvent` |
| 변경·저장 정책 | `src/persistence/notebook`의 `request_create`·`request_delete`·`flush`·`tick`·`needs_tick` |
| 데이터 불변식 | `src/persistence/domain`의 note·links·canvas / `valid_position` |
| 파일·writer | `src/persistence/storage`의 `Store`·`SaveQueue`·`data_path` |

스레드·스냅샷 흐름은 C3 저장 관심사로 검색한다. 공개 경계 강제: ADR-0010·`architecture_tests`; 저장·복구 계약: SAVE-001, SAVE-002, DATA-001, DATA-002.
