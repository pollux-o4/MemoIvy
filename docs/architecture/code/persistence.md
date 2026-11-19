# C4 · 저장 코드 탐색

| 관심사 | 탐색 키 |
|---|---|
| 명령·데이터 규칙 | `engine/src/commands.rs`의 `dispatch`, `domain.rs`의 메모·캔버스 유효성 검사 |
| 읽기·갱신 | `sqlite_store`의 `queries`, `revision::write_tx`, `schema` |
| 첨부 | `commands/attachments.rs`의 가져오기·읽기·삭제 |
| 이전 데이터 | `legacyNotesPath`, `sqlite_store/legacy.rs`의 `import`·`backup_path` |

저장·복구 계약은 DATA-001~003, OPS-002로 찾는다.
