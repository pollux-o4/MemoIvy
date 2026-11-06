# PERF-001 · 기존 합성 probe 측정

- 검증: PERF-001
- 날짜·환경: 2026-10-29, debug 빌드, Xvfb. 이번 문서 정비 이전의 측정 기록을 이관했다.
- 조건: 100개×1KiB 메모, 10KiB TextBuffer, 2ms 간격 100회 삽입, writer 500ms 지연, 시작 시 저장 1회 진행 중.
- 결과: callback p95 0.021ms, 5ms heartbeat 최대 간격 5.099ms, 저장 2회, 전체 1028.4ms.
- 방법: 실제 SaveQueue를 포함하고 저장 함수만 지연·횟수 측정용으로 대체했다. callback은 문자열 추출·메모 갱신·revision·상태 라벨 갱신을 재현했다.
- 미검증: 앱의 전체 mark·카드 갱신·화면 렌더링·창 이동·입력→프레임·실제 디스크·Wayland. 저장 내구성 증거는 DATA-002 검사로 구분한다.
- 실행 방법: OPS-003.
