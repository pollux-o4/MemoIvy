# 규칙 적용 검증 · 2026-10-29

- 검증: SAVE-001, SAVE-002, NOTE-001, NOTE-002, NOTE-003, DATA-001, DATA-002, PERF-001, UI-001
- 변경: 완료 계획·리뷰 5개 제거, 결정·계약·운영 문서 분리, 내부 경로 링크 제거, 기존 검사의 ID 대응.
- 환경: Rust 1.99.1, GTK 4.22, 별도 개발 패키지 경로, 격리 D-Bus·Xvfb·Openbox·합성 XDG 데이터.
- 실행: fmt, 단위 테스트 10개, clippy `-D warnings`, debug 빌드, probe 빌드, SAVE-001 UI smoke, PERF-001 probe 통과.
- 결과
  - SAVE-001: revision 단위 검사와 마지막 입력 직후 닫기·재실행·재열기 통과.
  - SAVE-002: 큐 실패→dirty 유지→복구→재시도 단위 검사 통과.
  - NOTE-001: 생성·닫기·재열기 통과. 동일 창 재사용·다중 창·목록 숨김은 이번에 미실행.
  - NOTE-002: 삭제 snapshot 후 다른 편집 보존 단위 검사 통과. 확인창 취소·삭제 실패 GUI는 이번에 미실행.
  - NOTE-003: 이번에 실행한 검색 전용 검사는 없다.
  - DATA-001: roundtrip·손상 파일·중복 ID·잘못된 색상 보존 검사 통과.
  - DATA-002: 신규·재저장 권한과 저장 실패 대상·임시 파일 검사 통과. 교체 후 sync 실패 주입은 미실행.
  - UI-001: 전체 내장 SVG의 1×/2× decode 통과. 테마·포커스·창 조작 GUI는 이번에 미실행.
  - PERF-001: 재현 callback p95 0.020ms, 5ms heartbeat 최대 간격 5.110ms, 100회 입력·500ms writer 지연에서 저장 2회.
- 성능 범위: 실제 SaveQueue와 합성 callback. 전체 GTK mark·카드 갱신·프레임·실제 디스크 측정은 제외된다.
- UI smoke 보완: 창이 표시되기 전 포커스 요청 race를 줄이도록 visible 창만 탐색하고, 창 관리자 시작 실패를 확인한다. 외부 테스트 도구의 라이브러리·테마 경로를 지정했다.
- GNOME/Wayland·화면 읽기 프로그램·IME 조합 입력은 이번에 검증하지 않았다.
