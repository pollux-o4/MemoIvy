# 백엔드 리팩터링 계획

- 범위: `src/persistence`, 그리고 지금 프런트 쪽에 있는 저장 규칙(`src/app/controller.rs`의 디바운스·트랜잭션).
- 상태: B-1~B-5 구현 완료. 공개 계약 변경은 2026-10-30 사용자 승인. 구현 계약은 persistence 루트, 협업 경계는 ADR-0010으로 검색.
- 계획 기준: `feat/canvas-experience` 브랜치 `e5b59a3`. 사용자 요청에 따라 같은 브랜치에서 구현했다.
- 경계 계약은 '프런트와의 계약'이 원본이다.

## 목표

프런트가 저장 절차를 몰라도 메모를 읽고 바꿀 수 있게 한다. 저장 형식과 저장 동작은 바꾸지 않는다.

완료 판단: `persistence` 밖에서 `SaveQueue`, `revision`, 트랜잭션을 직접 다루는 코드가 없다.

## 현재 문제

| # | 문제 | 근거 | 위반 원칙 |
|---|---|---|---|
| B1 | 저장 절차가 프런트에 흩어져 있다. `retry()` 후 `dispatch()` 순서를 호출자가 안다 | `editor.rs:162,199`, `library.rs:155`, `canvas/mod.rs:228`, `panels/mod.rs:106` | 정보 은닉, DRY |
| B2 | 저장 정책이 UI 조율 코드에 있다. 300ms·1초 디바운스, 트랜잭션 시작 | `controller.rs:107-124` | 관심사 분리, GRASP Information Expert |
| B3 | 생성·삭제 대기 상태를 프런트가 소유한다 | `state.rs`의 `Operation`, `menus.rs:109`, `library.rs:126` | 높은 응집도 |
| B4 | 큐 API가 내부 표현을 드러낸다. 트랜잭션을 `u64::MAX`로 구분하고 `poll`이 `(bool, Result)`를 돌려준다. 상태도 `clean`·`failed`·`in_flight` 조합으로 해석해야 한다 | `save_queue.rs:46-83` | 정보 은닉, 의도 드러내기 |
| B5 | 내부 데이터가 전부 공개돼 있다. `Store.notes`와 `Note`의 모든 필드를 어디서나 고친다 | `store.rs:20-50`, `editor.rs:85,173` | 불변식 보호, 데메테르 법칙 |
| B6 | 화면 규칙이 백엔드에 있다. 카드 크기, 빈 자리 배치, 한글 색상 이름 | `canvas.rs:7-8,18-39`, `store.rs:11-17` | 관심사 분리, 의존 방향 |
| B7 | `persistence::canvas`에 도메인 규칙과 저장 규칙이 섞여 있다 | `canvas.rs` 전체 | 단일 책임 |
| B8 | 오류 타입이 `io::Error`와 `String`으로 나뉜다 | `store.rs:52,82`, `save_queue.rs:53` | 일관성 |
| B9 | 쓰이지 않는 공개 API를 `allow(dead_code)`로 숨긴다. `Store::save`와 `Store.path` | `store.rs:47,77` | YAGNI |

## 목표 구조

```text
src/persistence/
  mod.rs        # 공개 면: Notebook, NoteView류 타입, 이벤트만 내보냄
  notebook.rs   # 파사드. 변경·저장·트랜잭션 조율
  note.rs       # Note 모델과 불변식
  links.rs      # 연결 그래프 규칙 (현 canvas.rs의 connect/remove/valid)
  store.rs      # 파일 형식·원자 저장
  save_queue.rs # writer 스레드 (모듈 내부 전용으로 축소)
  paths.rs
```

폴더 이름 `persistence`는 유지한다. 바꿔도 얻는 것이 적고 문서 참조만 깨진다.

## 프런트와의 계약 (승인된 설계 초안)

공개 인터페이스 변경은 승인받았다. 아래는 설계 초안이다. 실제 API는 `Notebook` 정의가 원본이며 색상은 기존 문자열 식별자를 검증해 사용한다.

```rust
pub struct Notebook { /* 비공개 */ }

impl Notebook {
    pub fn open(path: PathBuf) -> Result<Self, LoadError>;

    // 읽기
    pub fn notes(&self) -> &[Note];
    pub fn note(&self, id: Uuid) -> Option<&Note>;
    pub fn status(&self) -> SaveStatus;       // Saved | Pending | Saving | Failed(msg)
    pub fn busy(&self) -> bool;               // 생성·삭제 진행 중

    // 쓰기: 변경하면서 저장 예약까지 처리
    pub fn set_text(&mut self, id: Uuid, text: String);
    pub fn set_color(&mut self, id: Uuid, color: NoteColor);
    pub fn edit_canvas(&mut self, edit: impl FnOnce(&mut CanvasEdit)) -> bool; // 변경 여부 반환
    pub fn request_create(&mut self, position: Option<[f64; 2]>);
    pub fn request_delete(&mut self, id: Uuid);

    // 저장 제어
    pub fn flush(&mut self);                  // retry + dispatch를 대체
    pub fn tick(&mut self, now: Instant) -> Vec<NotebookEvent>;
    pub fn needs_tick(&self) -> bool;         // 프런트 타이머를 멈춰도 되는지
}

pub enum NotebookEvent {
    Saved,
    SaveFailed(String),
    Created(Uuid),
    Deleted(Uuid),
    OperationFailed(String),
}
```

- 스레드: `Notebook`은 GTK 스레드에서만 쓴다. writer 스레드는 내부 구현이다.
- 시간: `tick`이 `now`를 받아 디바운스를 화면 없이 시험할 수 있게 한다.
- 프런트는 GTK 타입을 넘기지 않고, 백엔드는 GTK를 모른다. `architecture_tests`가 이미 강제한다.

## 단계

각 단계는 독립 PR이고 끝날 때마다 기존 테스트가 통과해야 한다. 저장 형식은 어느 단계에서도 바뀌지 않는다.

### B-1. 현재 동작 고정 (인터페이스 변경 없음)

- 디바운스(300ms·1초 상한), 트랜잭션 후 최신 스냅샷 병합, 실패 후 재시도, 닫기 flush를 특성 테스트로 고정한다.
- 지금은 일부가 스모크로만 검증된다. 시간이 하드코딩돼 있어 이 단계는 기존 큐 API 수준까지만 시험하고, 디바운스는 B-3에서 옮긴 뒤 시험한다.
- 완료: `save_001`, `save_002`, `note_002` 사례가 화면 없이 돈다.

### B-2. 내부 정리 (인터페이스 변경 없음)

- `canvas.rs`의 그래프 규칙을 `links.rs`로, `Note`를 `note.rs`로 나눈다. 기존 경로는 재수출로 유지한다.
- `store.rs`의 쓰이지 않는 `Store::save`·`Store.path`를 정리한다. 단, 테스트가 쓰고 있어 테스트 쪽 사용을 `store::save` 호출로 함께 바꾼다.
- 큐 내부의 `u64::MAX`를 `enum Job { Snapshot(u64), Transaction }`으로 바꾼다.
- 완료: 동작 변화 없음, 외부 호출부 수정 없음.

### B-3. `Notebook` 추가 (신규 공개 API, 사용자 확인 후)

- `controller.rs:107-124`의 디바운스와 트랜잭션 규칙, `Operation`을 `Notebook`으로 옮긴다.
- 기존 공개 API는 아직 지우지 않아 프런트가 단계적으로 옮길 수 있게 한다. Expand-Contract 방식이다.
- 테스트: 가짜 시각으로 디바운스 경계, 진행 중 생성·삭제, 실패 복구를 시험한다.
- 완료: `Notebook` 단독 테스트 통과. 이 시점에 프런트 F-5가 시작될 수 있다.

### B-4. 화면 규칙 넘겨주기 (사용자 확인 후, 프런트와 동시 진행)

- 카드 크기 `WIDTH`·`HEIGHT`, `slot`, `vacant`, 한글 색상 이름을 프런트로 옮긴다.
- 백엔드에는 색상 식별자 `NoteColor`와 좌표 허용 범위 `LIMIT`만 남긴다.
- 주의: 지금 `Store::load`는 위치가 없는 옛 메모에 인덱스 기반 위치를 저장한다(`store.rs:70-74`). 이를 그냥 없애면 메모를 지울 때 옛 메모 위치가 밀린다. 그래서 프런트가 시작 시 한 번 빈 위치를 채워 저장하는 단계(F-5)와 같은 변경 단위로 묶는다.
- `canvas_001_legacy_and_graph_roundtrip` 테스트 기대값이 함께 바뀐다.

### B-5. 공개 면 축소 (프런트 이관 완료 후)

- `SaveQueue`, `Store`, 공개 필드 쓰기를 모듈 내부로 숨긴다. `Note`는 읽기 접근자만 남기거나 `pub(super)` 필드로 바꾼다.
- `architecture_tests`에 "`persistence` 밖에서 `SaveQueue`·`Store` 사용 금지"를 추가한다.
- 오류를 `LoadError`·`SaveError`로 통일하되 사용자 메시지 문구는 프런트에 둔다.
- 완료: B1~B9 해소, `DATA-001`·`SAVE-001`·`SAVE-002` 계약 변화 없음.

## 하지 않는 것

- 크레이트 분리: 경계가 안정된 뒤 다시 본다. 지금은 비용이 이득보다 크다(ADR-0010).
- `NoteRepository` 트레이트: 구현이 하나뿐이고 임시 파일로 충분히 시험된다. 저장소가 둘째로 필요해질 때 도입한다.
- 저장 형식 변경, SQLite 전환, 증분 저장.
- 비동기 런타임 도입: 현재 스레드 하나와 채널로 충분하다.

## 위험

| 위험 | 대응 |
|---|---|
| 저장 경계가 바뀌어 데이터 유실 | B-1 특성 테스트를 먼저 만든다. 실패 시 원본 보존 테스트(`data_001`, `data_002`)를 유지한다 |
| 닫기 flush 회귀 | 단계마다 `save_001_ui_smoke`, 합성 데이터로 재시작 후 내용 확인 |
| 옛 데이터 위치 변경 | B-4에서 옛 형식 고정 데이터로 전후 좌표를 비교한다 |
| 프런트와 작업 충돌 | Expand-Contract를 지키고, `controller.rs`는 한쪽만 수정 중임을 알린다 |
| PERF-001 저하 | 단계마다 합성 probe를 돌린다. `notes.to_vec()` 비용은 유지만 하고 줄이지 않는다 |

## 확정한 경계

- Notebook 파사드·Note 읽기 접근자·저장 정책 이관을 승인받았다.
- 창 조율은 프런트 shell이 소유한다. 백엔드 내부 모듈은 비공개다.
- 프런트·백엔드 동시 작업은 공개 계약을 유지하며 각자 소유 경로에서 수행한다. 가시성·architecture_tests로 경계를 강제한다.

## 이후 계획

리팩터링이 끝난 뒤, 필요가 확인될 때만 진행한다.

- 백업·복구 자동화: `OPS-002`의 수동 절차를 필요 시 앱 기능으로 올린다.
- 삭제 복구: 지금은 복구할 수 없다(`NOTE-002`). 사용자 요구가 있으면 휴지통 계약을 새로 쓴다.
- 저장 크기 대응: 메모 수가 늘어 전체 스냅샷 저장이 `PERF-001`을 넘으면 증분 저장을 검토한다.
- 크레이트 분리 재검토: `Notebook` API가 두 릴리스 동안 안정되면 판단한다.
