# 책임과 단일 진실 공급원 검토

## 발견과 변경
- SRP: main.rs 780줄이 앱 시작, 창 장식, 목록, 편집, 메뉴, 비동기 저장 완료를 모두 처리했다. main은 진입점만 남기고 app의 chrome/cards/library/editor/menus/controller 모듈로 분리했다. app/mod.rs는 비공개 앱 상태와 생명주기만 조립한다.
- SSoT: UI와 SVG가 별도로 다크 모드를 판정했다. theme.rs의 단일 판정을 양쪽에서 재사용한다.
- SSoT: 색상 값이 저장 검증, 기본값, 팔레트에서 중복됐다. store::COLORS를 공유하고 CSS는 표현만 담당한다.
- 중복: 일반 저장과 생성·삭제 저장의 실패 표시/잠금 해제가 복사돼 있었다. AppState::save_failed로 통합했다.
- 캡슐화: SaveQueue의 revision/durable/in_flight/failed가 외부에서 변경 가능했다. 내부 필드를 비공개로 하고 조회·재시도·실패 기록 메서드로 제한했다.
- 인지 부하: 창의 4원소 튜플을 EditorWindow로 바꾸고 상태 표시를 메서드로 모았다. 저장 완료 처리와 타이머 스케줄링은 별도 함수로 분리했다.

## 패턴 선택
- Mediator: controller는 worker 완료, 창 닫기, 생성·삭제 흐름을 조율한다. 개별 뷰가 저장 worker를 직접 운용하지 않는다.
- Observer: GTK 이벤트/테마 변경 구독을 유지한다. 별도 이벤트 버스는 만들지 않는다.
- Command: 기존 생성/삭제 Operation enum과 순차 저장 큐를 유지한다. 불필요한 명령 클래스 계층은 추가하지 않는다.
- LSP/ISP: 상속·큰 공개 trait 계약이 없는 현재 Rust 앱에서 구체적인 위반 근거는 없다. 패턴 적용을 위해 trait와 상속 계층을 만들지 않는다.
- DIP: GTK 없는 SaveQueue/Store와 GTK 조율을 분리한다. 저장 구현이 한 개이므로 가상의 여러 저장소를 위한 추상화는 추가하지 않는다.

## 보존 계약과 검증
JSON 형식, 0600, 원자 교체, 단일 writer, revision 확인, 닫기 flush, 실패 재시도 및 화면 외관을 유지한다. 기존 저장/큐/SVG 테스트 10개와 fmt/clippy/debug·release build를 통과했다. 격리 X11 UI 재검증은 테스트 창 관리자/포털 초기화 및 창 탐색 문제로 완료하지 못했으며 성공으로 기록하지 않는다. 실제 Wayland/IME의 기존 검증 한계는 README를 따른다.
