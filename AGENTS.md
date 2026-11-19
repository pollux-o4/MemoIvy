# MemoIvy 작업 지침

- 문서 규칙: `docs/rules`에서 RULE ID로 검색.
- "프로젝트 정리 진행": `docs/rules` 기준 문서 정리(없으면 `~/.codex/init-rules` 복사). 변경 동작은 명세(검증 테스트 ID 연결)·C4 코드 지도에 반영, 공문서체(번역체·수동태 지양), 기존 원본 규칙 문서는 수정 금지.
- 프로젝트 문서: `docs`에서 관심사로 검색.
- 개발 검증: 운영 문서에서 `verification`으로 검색.
- 데스크톱 이관 개발·검증: OPS-004로 검색.
- Electron 자동 조작·캡처: `mise run test:e2e` 또는 같은 격리(`env -u WAYLAND_DISPLAY`, `--ozone-platform=x11`, `xvfb-run`)로만. Wayland에서 `xvfb-run`만으로는 실제 화면에 창이 떠 입력을 가져간다.
- 테스트 실행: 맨 `cargo test` 금지(사용자 화면에 GTK 창이 뜬다). 빌드 확인·목록·시간 측정 포함 항상 `mise run test:<영역>` 또는 `mise run test <이름>`. 바뀐 영역만, 전체 실행은 요청 시에만. 빌드만은 `cargo test --no-run`.
- UI 변경: 실행 화면부터 확인하고 관련 코드만 읽기.
- 모듈 공개 인터페이스 변경: 구현 전 사용자 확인.
- UI 설계·구현·리뷰: UI-001을 먼저 읽고 공통 펼침 제어를 재사용한다.
- UI 이관·적대 검수: UI-REF-001의 GTK 원본 캡처와 같은 데이터·테마·창 크기로 비교한다. 기준 이미지는 명시적 요청 없이 갱신하지 않는다.

<!-- ltm -->
과업지시서·프롬프트 작성 요청은 `.ltm/AGENTS.md`를 따른다.
