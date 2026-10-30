# 프런트엔드 리팩터링 계획

- 범위: `src/app`, `src/presentation`, `tools/behavior`.
- 상태: F-1~F-9 구현 완료. 인터페이스·폴더 변경과 프런트/백엔드 모듈러 모놀리스 경계는 2026-10-30 사용자 승인.
- 계획 기준: `feat/canvas-experience` 브랜치 `e5b59a3`. 사용자 요청에 따라 같은 브랜치에서 구현했다.
- 짝 문서: `rooms/back/plan.md`. 저장 경계 계약의 원본은 그쪽이다.

## 목표

기능별로 찾아 고칠 수 있게 하고, 화면 간 의존을 한 방향으로 만든다. 사용자에게 보이는 동작과 모양은 바꾸지 않는다.

완료 판단: 목록·편집기·캔버스 중 하나를 고칠 때 다른 기능의 내부를 읽지 않아도 된다.

## 현재 문제

| # | 문제 | 근거 | 위반 원칙 |
|---|---|---|---|
| F1 | 의존 방향이 순환한다. 공용 위젯이 창을 부른다 | `widgets/cards.rs:2,33,109,118` → `windows`, `controller.rs:3-4` | 낮은 결합, 의존 방향 |
| F2 | 기능 코드가 공용 폴더에 있다. 목록 전용 `cards`, 편집기 전용 `live_editor`·`formatting` | `src/app/widgets/` | 높은 응집도 |
| F3 | `AppState`가 데이터, 저장, 창 위젯, 닫기 플래그를 모두 가진다 | `state.rs:26-44`, 필드 추가 시 생성 코드 3곳 수정 (`library.rs:90`, `canvas/tests.rs:97,396`) | 단일 책임 |
| F4 | 버튼 클릭을 명령 전달 수단으로 쓴다. `s.add.emit_clicked()` | `shortcuts.rs:216`, `canvas/mod.rs:124`, `panels/toolbar.rs:248` | Command, 관심사 분리 |
| F5 | 프런트가 `Note`를 직접 읽고 고친다. 캔버스만 `port`를 거친다 | `editor.rs:85,173`, `cards.rs:3`, `library.rs:6` | 정보 은닉, 일관성 |
| F6 | `CanvasView` 하나에 필드 약 40개가 있다. 위젯, 세션, 조작 상태가 한데 섞였다 | `canvas/mod.rs` | 단일 책임, God Object |
| F7 | 긴 함수가 조립·동작·문구를 함께 담는다 | `editor.rs` `open_note` 약 200줄, `live_editor.rs` `attach` 약 220줄, `library.rs` `build` | SRP, 가독성 |
| F8 | 닫기 상태가 플래그 조합이다. `bypass`, `closing`, `closing_library` | `controller.rs:55,127-153`, `editor.rs:182`, `library.rs:133-151` | 상태 모델 명시 |
| F9 | 같은 값·코드가 반복된다. `ZOOM_STEP` 두 곳, 100% 보기 두 곳, 생성·삭제 시작 시 버튼 비활성화 두 곳 | `keyboard.rs:11`, `toolbar.rs:11`, `menus.rs:110`, `library.rs:127` | DRY |
| F10 | 쓰지 않는 잔여물 `let _a = app.clone()` | `library.rs:120`, `menus.rs:140` | KISS |
| F11 | 모든 GTK 테스트가 서식 테스트 안에서 실행된다 | `live_editor.rs:495-499` | 테스트 구조 |
| F12 | 캔버스 색이 CSS 토큰과 별도로 코드에 있다. 다크 테마 강조색과 어긋난다 | `paint.rs:13-15`, `tokens-dark.css:10` | 단일 원본 |
| F13 | 16ms 타이머가 할 일이 없어도 계속 돈다 | `controller.rs:29` | 자원 낭비 |

## 목표 구조

```text
src/app/
  mod.rs            # 앱 시작
  shell/            # 창 간 조율 (현 state + controller의 남는 부분)
    state.rs        # AppState: Notebook + Views
    commands.rs     # new_note, delete_note, open_note, close 요청
    views.rs        # 데이터 변경 후 어떤 창을 갱신할지
    lifecycle.rs    # 닫기 상태 모델
  library/          # 목록 창 + cards
  editor/           # 편집 창 + live_editor + formatting
  canvas/           # 현 windows/canvas 이동
  shared/           # chrome, menus, shortcuts, focus
```

의존 방향은 다음 한 가지만 허용한다.

```text
library, editor, canvas → shell(commands), shared, presentation, persistence 공개 면
shell → library, editor, canvas의 공개 함수 (창 열기·갱신)
library ↛ editor ↛ canvas (서로 직접 호출 금지)
```

`shell`과 기능 모듈 사이의 왕복 호출은 함수 호출로 유지한다. 이벤트 버스를 두면 흐름만 숨기고 얻는 것이 적다.

## 단계

각 단계는 독립 PR이다. 끝날 때마다 `OPS-003` 기본 검사와 해당 스모크를 돌린다. UI를 건드리는 단계는 `AGENTS.md` 규칙대로 실행 화면부터 확인한다.

### F-1. 안전망과 잔여물 정리 (인터페이스 변경 없음)

- GTK 테스트 실행기를 `src/app/gtk_tests.rs`로 분리한다 (F11).
- `AppState` 생성을 테스트 픽스처 함수 하나로 모은다 (F3의 3곳 수정 문제).
- `let _a`, 중복 상수와 100% 보기를 정리한다 (F9, F10).
- 완료: 동작 변화 없음, 테스트 이름 유지.

### F-2. 명령 함수 도입 (내부 인터페이스, 사용자 확인 후)

- `commands::new_note(state)`, `commands::delete_note(state, id)`를 만든다.
- `add.emit_clicked()` 세 곳과 버튼 비활성화 두 곳을 이것으로 바꾼다 (F4, F9).
- 완료: 버튼은 명령을 부르기만 하고, 명령이 버튼을 부르지 않는다.

### F-3. 의존 방향 바로잡기 (내부 인터페이스, 사용자 확인 후)

- 목록 재생성과 캔버스 갱신 호출을 `views::refresh_all`로 옮겨 `cards.rs`가 `windows`를 모르게 한다 (F1).
- 카드 클릭은 `commands::open_note`로 보낸다.
- `architecture_tests`에 "`shared`는 기능 모듈을 참조하지 않는다", "기능 모듈끼리 직접 참조하지 않는다"를 추가한다.

### F-4. 폴더 이동 (경로만 변경)

- 목표 구조로 파일을 옮긴다. 다른 수정과 섞지 않아 `git log --follow`로 추적할 수 있게 한다 (F2).
- `docs/architecture`와 ADR-0010의 경로를 같은 PR에서 고친다.
- 다른 담당자와 충돌하기 쉬우므로 가능한 한 짧게, 작업 시점을 맞춰 진행한다.

### F-5. `Notebook`으로 이관 (백엔드 B-3 이후, 사용자 확인 후)

- 저장 호출을 옮긴다. `queue.retry(); dispatch()` 다섯 곳을 `notebook.flush()`로 바꾼다.
- `store.notes` 직접 수정을 `set_text`, `set_color`, `edit_canvas`로 바꾼다 (F5).
- 캔버스 `port.rs`는 유지하되 `Notebook`만 보도록 줄인다.
- `controller::tick`은 `notebook.tick()` 이벤트를 화면에 반영하는 일만 남긴다.
- `needs_tick()`이 거짓이면 타이머를 멈춘다 (F13). 다음 변경에서 다시 켠다.
- 카드 크기와 빈 자리 배치를 `canvas/model`로 받고, 옛 메모의 빈 위치를 시작 시 채운다 (백엔드 B-4와 같은 변경 단위).

### F-6. `AppState` 나누기 (내부 인터페이스, 사용자 확인 후)

- `LibraryView`(list, count, search, error, add)와 `windows`, `canvas`를 `Views`로 묶는다 (F3).
- 닫기 플래그를 하나의 상태로 바꾼다 (F8).

```rust
enum Closing { None, Editors(Vec<Uuid>), Library }
```

- `bypass`는 '이미 저장을 마쳤으니 바로 닫기'라는 뜻이므로, 닫기 요청 함수의 인자로 바꾸는 안을 우선 검토한다.
- 검증: `save_001`, `focus_001`, 저장 실패 중 닫기 복구.

### F-7. `CanvasView` 나누기 (캔버스 공개 함수 변경 없음)

- 필드를 `CanvasWidgets`(위젯), `BoardSession`(선택·카메라·표시 위치), `History`로 묶는다 (F6).
- `History`는 GTK 없는 제네릭 구조로 `model`에 두고 undo·redo·상한을 단위 테스트한다.
- 입력 처리와 패널은 필요한 묶음만 받게 해 데메테르 법칙 위반을 줄인다.
- 완료: 외부 공개 함수 4개 유지, `tests.rs` 시나리오 무변경 통과.

### F-8. 긴 함수 나누기 (인터페이스 변경 없음)

- `open_note` → 창 구성, 팔레트, 메뉴, 저장 연결 (F7).
- `attach` → 태그 만들기, 입력 처리, 제스처, 원문 전환. 이 파일은 `FMT-001` 위험이 가장 크므로 동작별 테스트를 먼저 보강한다.
- `build` → 데이터 불러오기, 오류 화면, 창 구성. 남은 `MessageDialog`도 여기서 공용 확인창으로 교체한다.

### F-9. 캔버스 색 일원화 (F12, 선택)

- 그리기 색을 CSS 토큰에서 읽어 다크 테마 강조색 불일치를 없앤다.
- 실제로 눈에 띄는 차이가 있을 때만 진행한다. 다만 이 항목은 수정 전후 화면 비교가 필요하다.

## 권장 순서

```text
F-1 → F-2 → F-3 → F-4 ─┐
백엔드 B-1 → B-2 → B-3 ┴→ F-5 + B-4 → B-5 → F-6 → F-7 → F-8 → F-9
```

F-4 전까지는 백엔드와 겹치는 파일이 적어 병행할 수 있다. F-5와 B-4는 함께 진행한다.

## 하지 않는 것

- libadwaita, Relm4 같은 프레임워크 도입.
- 이벤트 버스, DI 컨테이너, 전역 상태 저장소.
- 트레이트로 모든 뷰를 추상화하기: 구현이 하나뿐이다.
- `model` 등 이미 경계가 잘 잡힌 캔버스 내부 모듈의 재구성.
- 시각 디자인 변경. 이 계획은 동작과 모양을 그대로 둔다.

## 위험

| 위험 | 대응 |
|---|---|
| 폴더 이동으로 진행 중인 작업과 충돌 | F-4를 경로 변경 전용 PR로 짧게 처리 |
| 소유권 변경으로 `RefCell` 이중 대여 패닉 | F-6에서 대여 범위를 함수 안으로 줄이고 스모크 반복 |
| 닫기·저장 회귀 | `save_001`·`key_001` 매 단계 실행, 실패 중 닫기 사례 추가 |
| 포커스 회귀 | 이전 먹통 버그 순서(패널 메뉴 → Esc → 도움말 → Ctrl+W)를 스모크에 고정 |
| 성능 회귀 | F-5, F-7 뒤 `PERF-002` 재측정 |
| 좌표 기반 스모크가 레이아웃 변경에 깨짐 | 이 계획은 레이아웃을 바꾸지 않는다. 깨지면 회귀로 본다 |

## 확정한 경계

- 기능별 폴더와 shell 명령·갱신·창 수명 조율을 승인받았다.
- 프런트는 Notebook 공개 계약만 사용한다. 기능 간 직접 참조와 shared의 기능 참조는 architecture_tests로 금지한다.
- 생성·삭제 버튼 상태는 명령과 저장 이벤트로 갱신한다. Closing의 Committed 상태는 저장 완료 후 재진입을 구분한다.
- F-9는 어두운 화면에서 강조색 불일치를 확인해 CSS 토큰 조회로 통합한다.

## 이후 계획

리팩터링이 끝난 뒤, 필요가 확인될 때만 진행한다.

- 미검증 항목 확인: 실제 Wayland·한글 IME·GPU 프레임률, 사용성 관찰(`UX-001`).
- 캔버스 기능: 실제 사용에서 요구가 확인되면 빈 곳 더블클릭 새 메모, 우클릭 메뉴, 등간격 맞춤선을 검토한다.
- 메모리: 300개에서 늘어난 약 20MB의 원인을 분리하고, 필요하면 카드 위젯 지연 생성을 검토한다.
- 대량 재배치: 전 카드가 움직일 때 37fps인 구간을 실제 GPU에서 다시 재고 판단한다.
- 스타일 회귀: 필요해지면 스크린샷 비교 테스트를 도입한다.
