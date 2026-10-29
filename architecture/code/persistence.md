# C4 · 저장·닫기 코드 라우팅

[상위: 저장 컴포넌트](../03-persistence-components.md) · [UI 코드](ui.md)

| 관심사 | 소스 | 시작 지점 |
|---|---|---|
| 변경 표시·저장 예약 | [controller.rs](../../src/app/controller.rs) | `mark`, `dispatch` |
| 주기적 처리·작업 결과·닫기 완료 | [controller.rs](../../src/app/controller.rs) | `start` → `tick`, `process_result` |
| 편집 창의 닫기 요청 | [editor.rs](../../src/app/editor.rs) | `open_note` 안 `connect_close_request` |
| 목록 창의 닫기 요청 | [library.rs](../../src/app/library.rs) | `build` 안 `connect_close_request` |
| 실패 공통 복구 | [app/mod.rs](../../src/app/mod.rs) | `AppState::save_failed` |
| worker 생성·직렬 저장·채널 | [save_queue.rs](../../src/save_queue.rs) | `SaveQueue::new`, `dispatch`, `transaction`, `poll` |
| 최신 revision·재시도·닫기 가능 여부 | [save_queue.rs](../../src/save_queue.rs) | `changed`, `retry`, `record_failure`, `clean` |
| 메모 모델·색상 목록·로드 검증 | [store.rs](../../src/store.rs) | `Note`, `COLORS`, `Store::load` |
| 파일 원자 저장 | [store.rs](../../src/store.rs) | `save` |
| 경로 선택 | [paths.rs](../../src/paths.rs) | `data_path` |

## 검증으로 들어가기

- [저장 테스트](../../src/store.rs): 손상 데이터 보존, ID/색상 검증, roundtrip, 0600 권한, 실패 시 기존 데이터 유지.
- [큐 테스트](../../src/save_queue.rs): 최신 revision 유지, 실패 재시도, 작업 후 다른 편집 보존, burst 병합.
- [GTK smoke test](../../tools/ui_smoke.py): 생성·입력·즉시 닫기·종료·재실행·다시 열기.
- [성능 probe](../../tools/performance-probe/src/main.rs): 합성 부하의 큐 동작과 main-loop 지연. 실제 화면 전체의 입력 지연 측정과 동일하지 않다.

구체적 테스트 실행 환경과 기존 미검증 범위는 [README](../../README.md)의 개발 검증 절을 따른다.
