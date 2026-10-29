# Code · 저장 탐색

- 저장 예약·결과·닫기: controller의 `mark`, `dispatch`, `process_result`, `tick`.
- 실패 표시·편집 복구: `AppState::save_failed`.
- worker·채널·revision: `SaveQueue`의 `new`, `dispatch`, `transaction`, `poll`, `clean`.
- 모델·로드 검증: `Note`, `COLORS`, `Store::load`.
- 파일 저장: store의 `save`.
- 데이터 위치: `data_path`.

계약 ID로 테스트 검색: SAVE-001, SAVE-002, DATA-001, DATA-002, PERF-001.
