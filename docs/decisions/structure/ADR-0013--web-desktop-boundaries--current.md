# ADR-0013 · 웹 화면과 Rust 데이터 엔진

- 맥락: GTK 화면 검수 비용과 Windows·macOS·Linux 지원 요구. SQLite 전환·조용한 자동저장·역할 검색 가능한 경계는 MIG-001의 사용자 피드백을 따른다.
- 결정: 한 제품·한 설치 패키지 안에서 React 화면, Electron 창·OS 통합, Rust 원본·도메인·SQLite 저장을 분리한다. 기능별 모듈형 모놀리스와 외부 경계의 adapter를 사용한다.
- 실행: Electron main이 엔진 자식 프로세스 하나를 소유하고 모든 창이 공유한다. 줄 단위 JSON 요청·응답·변경 이벤트를 stdio로 전달한다. 로그는 stderr다.
- 계약: engine_contract의 version·요청 ID·revision·savedRevision으로 명령 결과와 저장 완료를 구분한다. 충돌은 최신 snapshot과 함께 보고하고 초안은 화면에 유지한다. 비멱등 명령을 자동 재전송하지 않는다.
- 저장: SQLite가 commit·rollback을 맡고 엔진은 기존 메모·포함 관계·연결 규칙을 유지한다. 이전 JSON은 무수정·백업 후 한 번 가져온다. 데이터 폴더 잠금은 기존 GTK 실행과 충돌도 차단한다.
- 화면: 앱과 Storybook이 같은 컴포넌트를 사용한다. 실제 desktop adapter와 mock adapter만 바꾼다. 입력 초안·포커스·선택·포인터 중 상태는 화면이, 확정 원본과 불변식은 엔진이 소유한다.
- 종료: 화면 초안 저장을 기다린 뒤 내부적으로 flush한다. 정상 편집과 닫기는 방해하지 않는다. 실패 시 초안 보존과 복구 조작을 제공한다.
- 이름: library·editor·canvas, sqlite_store·engine_transport·window_lifecycle처럼 책임을 검색할 수 있는 이름을 사용한다. 명명만을 위한 검사 체계는 추가하지 않는다.
- 대안: 전체 Clean Architecture 계층과 원격 서비스 분리는 현재 비용을 정당화하지 못한다. SQLite를 UI나 Electron에서 직접 다루면 데이터 규칙이 분산되므로 Rust만 소유한다.
- 전환: GTK 삭제는 기능 동등성과 복구 확인 후 진행한다. 기존 결정은 이관 전 비교 기준으로 남긴다. 검증과 실행 명령은 OPS-004 관심사로 찾는다.
