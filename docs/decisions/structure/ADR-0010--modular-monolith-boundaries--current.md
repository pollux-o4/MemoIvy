# ADR-0010 · 모듈러 모놀리스 경계

- 맥락: 프런트엔드와 백엔드를 동시에 변경해도 서로의 내부 구현에 의존하지 않아야 한다. 책임 분리는 ADR-0007을 잇는다.
- 대안: 크레이트 분리, 서비스 분리, 단일 공개 저장 모듈. 현재는 한 실행 파일·한 크레이트에서 비공개 모듈로 경계를 강제한다.
- 결정: 백엔드 공개 면을 persistence 루트의 Notebook·Navigation·읽기 모델·상태·이벤트로 제한한다. 큐·파일 저장·revision·트랜잭션은 내부 구현이다.

| 영역 | 경로 | 담당 |
|---|---|---|
| 백엔드 | `src/persistence` | 원본·불변식·저장 정책·파일 형식·writer |
| 프런트 조율 | `src/app/shell` | 명령·창 갱신·닫기·필요할 때만 폴링 |
| 프런트 기능 | `src/app/library`, `src/app/editor`, `src/app/canvas` | 기능별 화면·입력·세션 |
| 프런트 공통 | `src/app/shared`, `src/presentation` | 공유 조작·표현·테마·아이콘 |

- 협업 계약: 백엔드는 GTK·app·presentation을 참조하지 않는다. 프런트는 persistence 하위 모듈과 Note 필드 쓰기에 접근할 수 없다. 공개 계약 변경은 양쪽 호출부·테스트를 같은 변경 단위로 갱신한다.
- 기능끼리 창 열기·갱신은 shell commands/views로 보낸다. shared는 기능 모듈을 직접 호출하지 않는다. shell과 기능 사이의 왕복 함수 호출은 허용한다.
- 캔버스 저장 연결은 port가 맡는다. model은 GTK 없는 규칙과 History를 소유한다. 외부 화면 함수는 open·refresh·update_status·CanvasView::destroy이고, shell 보조 진입점으로 scope_for·show_project·adopt를 둔다. 포함 관계 규칙은 persistence domain::nesting이 소유한다.
- 강제: Rust 가시성과 architecture_tests로 백엔드 역의존·저장 내부 노출·기능 간 직접 참조·port 우회·model의 GTK 의존을 차단한다.
- 병렬 변경: 백엔드는 persistence, 프런트는 app/presentation을 담당한다. 저장 정책 변경과 화면 변경은 공개 계약이 같으면 독립적으로 구현한다. 큐 단위 PERF-001 probe만 저장 구현을 직접 포함하는 검증 도구다.
- 검증 분담: 백엔드는 `cargo test --locked persistence`, 프런트는 OPS-003 GTK·UI 검사로 확인한다. 양쪽 통합 시 기본 검사와 architecture_tests를 실행한다. 공개 계약 변경은 구현 전 확인하고 양쪽 담당이 호출부 이관을 맞춘다.
- 재검토: 공개 계약이 두 릴리스 동안 안정되면 크레이트 분리를 판단한다.

## 내부 책임 구분

- SRP: 캔버스 루트의 창 조립·변경·카메라·표시 조율을 controller 하위로 옮긴다. 루트는 기존 화면 계약과 세션 소유권을 유지한다.
- SSoT: 좌표 허용 규칙은 domain::canvas::valid_position이 소유하며, 편집과 저장 데이터 검증이 공유한다.
- 정보 은닉: persistence 내부는 domain·notebook·storage로 나누고 루트의 기존 공개 계약을 유지한다.
- 패턴: Notebook은 Facade, canvas port는 Adapter, History는 Snapshot/Memento 역할을 유지한다. 명령마다 클래스·trait를 추가하는 GoF Command 구현은 현재 단일 호출 경로에 필요하지 않다.
- OCP·DIP: UI의 저장 구현 직접 접근 금지를 유지한다. 구현 교체 요구가 없는 곳에는 추상화를 추가하지 않는다. 상속 계층이 없어 LSP를 적용할 구체적 대상은 없다.
- 검증: 기존 명세 테스트의 내용과 GTK 프로세스 격리를 유지하고, 소스 폴더의 파일 수는 architecture_tests로 5개 이하를 검사한다.

## 내부 경계의 적용

- 목록은 검색·갱신 조율, 카드 조립, 프로젝트 섹션, 본문 미리보기로 나눈다. 이름 없는 항목의 표시 제목은 목록 내부 한 함수가 소유한다.
- shell lifecycle은 타이머 조율, 저장 이벤트의 화면 반영, 저장 완료 후 종료, 회귀 시나리오로 나눈다. 기존 닫기·flush·wake 계약과 이벤트 처리 순서를 유지한다.
- 생성 메뉴는 shared의 조립·모핑 표면이 소유하며 생성 실행·허용 여부는 콜백으로 주입한다. busy와 도메인 생성 가능 여부를 합치는 화면 정책은 shell commands가 소유한다. Notebook의 깊이 불변식과 구분한다.
- Notebook의 CanvasEdit은 캔버스 변경 규칙을 맡는다. 내부 Operation의 동일 적용 함수를 저장 후보와 저장 성공 후 원본에 사용해 생성·삭제·프로젝트 승격 규칙의 분기를 막는다.
- 공유 메뉴는 행 표현·확인창·메모 메뉴로 나누고 기존 공통 진입점을 재노출한다. Facade·Adapter·Memento와 콜백 주입으로 현재 변경 지점을 감싼다. 단일 구현을 위한 trait·전략 클래스는 추가하지 않는다.
- paint는 배경·연결선 캐시·상호작용 HUD로 분리한다. 곡선 계산은 연결선과 HUD가 공유하고 기존 타일 캐시 진입점은 재노출한다.
- 모션은 보간·캐시·프레임 중 배치·유휴 콜백 종료를 기존 계약대로 보존한다. 모듈 분리가 프레임별 계산이나 위젯 생성을 추가해서는 안 된다. 검증 환경의 차이는 OPS-003에서 찾는다.
