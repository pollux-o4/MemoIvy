# 관심사별 계층화 · 2026-10-29

- 구조
  - src: app·persistence·presentation. app은 창·위젯·상태·조율을 분리.
  - docs: architecture·decisions·specs·operations·verification.
  - tools: behavior·quality. integration에 desktop 템플릿을 함께 배치.
- 검증
  - fmt·Clippy 경고 오류화·앱 빌드·단위 테스트 10개 통과.
  - SAVE-001: Xvfb·Openbox에서 생성·입력·즉시 닫기 저장·종료·재실행·재열기 통과.
  - PERF-001: 입력 100회, p95 0.021ms, heartbeat 최대 5.181ms, 쓰기 2회. 합성 writer 지연 500ms.
  - 설치 스크립트 구문·HOME 치환 desktop 검증 통과. 이동한 계약·결정·과거 검증 기록 및 주요 원본 내용 보존 확인.
- 범위: 개발 빌드 검증. 사용자 설치본 갱신·GNOME 화면 확인은 수행하지 않음.
