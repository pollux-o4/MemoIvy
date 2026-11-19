# RULE-SPEC · 계약 중심 명세

- 배치
  - 동작: `specs/behavior/SAVE-001--close-flush.md`
  - 데이터: `specs/data/DATA-001--notes-format.md`
  - 품질: `specs/quality/PERF-001--input-responsiveness.md`
- 계약 하나당 ID 하나, 관련 성공·실패 사례를 묶는다.
- 필수 항목
  - 공통: 범위·사전 조건·입력·기대 동작·실패 복구·관찰 가능한 완료 기준.
  - 데이터: 의미·검증·호환성.
  - 품질: 부하·환경·측정 방법·기준.
- 유지
  - 같은 계약은 ID를 유지해 갱신하고 요구 변경 근거를 남긴다.
  - 중요한 선택 이유는 ADR ID로 연결한다.
  - 폐기: `--retired.md`와 대체 ID 또는 폐기 이유. 과거 내용은 Git으로 보존한다.
- 검증 연결: RULE-TEST.
