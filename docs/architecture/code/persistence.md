# Code · 저장 탐색

- 앱 조율
  - controller의 `mark`, `dispatch`, `process_result`, `tick`.
  - state의 `AppState::save_failed`.
- 저장: persistence.
  - worker·채널·revision: `SaveQueue`의 `new`, `dispatch`, `transaction`, `poll`, `clean`.
  - 모델·로드·파일 저장: store의 `Note`, `COLORS`, `Store::load`, `save`.
  - 데이터 위치: paths의 `data_path`.
- 검증: SAVE-001, SAVE-002, DATA-001, DATA-002, PERF-001로 검색.
