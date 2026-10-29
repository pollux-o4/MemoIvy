# LSM 작업 지침

- 문서 정비: Codex 홈의 `init-rules`에서 RULE-CORE·RULE-ADR·RULE-SPEC·RULE-TEST·RULE-OPS를 찾는다.
- 탐색: 구조는 `docs/architecture`, 결정은 관심사별 `docs/decisions`, 계약은 `docs/specs`, 절차는 OPS ID로 찾는다.
- 검증: Rust 변경은 OPS-003, 저장·닫기 변경은 SAVE-001 UI smoke도 실행한다.
- `Note`, `COLORS`, `data_path`, `SaveQueue`가 각각 데이터·색상·경로·저장 상태의 구현 원본이다.
