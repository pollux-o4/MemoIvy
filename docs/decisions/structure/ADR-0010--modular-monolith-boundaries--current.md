# ADR-0010 · 모듈러 모놀리스 경계

- 맥락: 프런트엔드와 백엔드 담당이 항상 다르다. 한 실행 파일을 유지하면서 서로의 내부를 몰라도 변경할 수 있어야 한다. 책임 분리는 ADR-0007을 잇는다.
- 대안: 크레이트 분리, 서비스 분리, 단일 모듈 유지. 크레이트 분리는 경계가 가장 강하지만 AppState·GTK 스레드 공유 구조까지 다시 나눠야 해 이번 변경 범위를 넘는다.
- 결정: 한 크레이트 안에서 모듈 경계를 코드로 강제한다.

| 영역 | 경로 | 담당 |
|---|---|---|
| 백엔드 | `src/persistence` | 저장 형식·큐·그래프 규칙 |
| 조율 | `src/app/controller.rs`, `src/app/state.rs` | 뷰와 저장의 연결 |
| 프런트엔드 | `src/app/windows`, `src/app/widgets`, `src/presentation` | 창·위젯·스타일·아이콘 |

- 캔버스 내부 구성: `model`(순수 규칙) → `scene`·`panels`·`input`·`motion`(GTK) → `port`(저장 계층 유일 경계). 외부 공개는 `open`·`refresh`·`update_status`·`CanvasView::destroy`뿐이다.
- 스타일: `presentation/styles`의 토큰과 구성요소별 CSS로 나눈다.
- 강제: `architecture_tests`가 백엔드의 프런트엔드 의존, 캔버스의 `port` 우회, `model`의 GTK 의존을 실패시킨다.
- 결과: 저장 형식이 바뀌어도 캔버스는 `port`만 고친다. 목록·편집기는 아직 `Note`를 직접 읽는다. 해당 창을 크게 바꿀 때 같은 방식의 경계를 도입한다.
