# C4 Code · 저장 탐색

| 관심사 | 탐색 키 |
|---|---|
| 공개 경계 | persistence 루트의 `Notebook`·`Note`·`CanvasEdit`·`Navigation`, `SaveStatus`·`NotebookEvent`·`MAX_DEPTH` |
| 변경·저장 정책 | `src/persistence/notebook`의 `request_create`·`request_create_project`·`request_delete`·`flush`·`tick`·`needs_tick`, `Operation::apply`(후보·원본 공통 적용) |
| 캔버스 변경 | notebook canvas_edit의 `CanvasEdit` / 외부는 persistence 루트의 기존 재노출 계약 사용 |
| 데이터 불변식 | `src/persistence/domain`의 note·links·canvas·nesting / `valid_position`·`can_nest`·`lift_children` |
| 첨부 파일·사용처 | `src/persistence/attachments`의 `Attachments::reconcile`·`entries`·`delete`·`prune`, files의 `AssetStore::import`·`AssetId` |
| 방문 기록·바로가기 순위 | `src/persistence/navigation`의 `Navigation::beside`·`record`·`ranked` |
| 파일·writer | `src/persistence/storage`의 `Store`·`SaveQueue`·`data_path` |

스레드·스냅샷 흐름은 C3 저장 관심사로 검색한다. 공개 경계 강제: ADR-0010·`architecture_tests`; 저장·복구 계약: SAVE-001, SAVE-002, DATA-001, DATA-002, DATA-003(첨부).
