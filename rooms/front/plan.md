# 프런트엔드 리팩터링 계획

- 범위: `src/app`, `src/presentation`, `tools/behavior`. `src/persistence`는 수정하지 않는다.
- 기준: `feat/canvas-experience`의 링크 미리보기·목록 입력·카드 표식·캔버스 단축키 변경(FMT-003, CANVAS-004). 이 변경이 커밋된 뒤 시작한다.
- 이전 계획(F-1~F-9): 구현 완료. 내용은 Git 이력의 이 파일에 있다.
- 짝 문서: `rooms/back/plan.md`. 경계는 ADR-0010을 따른다.

## 목표

기능을 더 얹어도 한 파일·한 구조체가 커지지 않게 한다. 사용자에게 보이는 동작·모양은 바꾸지 않는다.

완료 판단: 목록 입력·링크 카드·서식 표시 중 하나를 고칠 때 나머지를 읽지 않아도 된다. 같은 테스트를 반복 실행해도 타이밍으로 실패하지 않는다.

## 현재 문제

| # | 문제 | 근거 | 위반 원칙 |
|---|---|---|---|
| R1 | 편집기 파일 하나가 표시·입력 연결·위젯 배치·테스트를 모두 가진다. 테스트 모듈 뒤에 본 코드가 이어진다 | `live_editor.rs` 1540줄, 테스트 600행부터, `create_tags` 1197행 | 높은 응집도, 가독성 |
| R2 | `LiveEditor` 필드가 20개를 넘고 파싱 결과·태그·위젯 풀·입력 상태·포인터 상태가 섞였다 | `live_editor.rs:34` | 단일 책임, God Object |
| R3 | `create_tags`가 이름 없는 태그 5개짜리 튜플을 돌려준다 | `live_editor.rs:1197` | 의도 드러내기 |
| R4 | 파서 파일이 인라인 파싱·목록 인식·주소 인식·링크 판별·카드 요약을 모두 가진다 | `markdown.rs` 1126줄 | 높은 응집도 |
| R5 | 항목 조회와 원문 조각 읽기가 호출부마다 반복된다 | `live_editor.rs`, `list_edit.rs`의 `items.iter().find`, `buffer.text(iter_at_offset…)` | DRY, 정보 은닉 |
| R6 | 링크 카드 상태가 `shown`·`pending`·`over_card`·`dismissed`와 타이머 4개의 조합이다 | `link_preview.rs:36-50` | 상태 모델 명시 |
| R7 | 링크 카드 API에 의미 없는 값을 넘긴다. 현재 링크를 읽는 코드도 반복된다 | `Rectangle::new(0, 0, 0, 0)`, `shown.borrow().as_ref().map` 5곳 | 인터페이스 분리, DRY |
| R8 | 기호 모양과 체크 모양의 원본이 둘이다 | `BULLETS`: `live_editor.rs:18`, `scene/preview.rs:9` / CSS: `.md-checkbox`, `.line-marker.task` | 단일 원본 |
| R9 | 선택된 노드를 모으는 코드가 반복되고 `delete_selected`가 UI 판단과 명령을 함께 가진다 | `canvas/mod.rs:439,512,540` | DRY, Command |
| R10 | 위젯 트리 탐색이 제각각이다 | `keyboard.rs:178`, `inspector.rs:100`, 테스트의 `named`·`button_with_class`·`count`·`find_checkbox` | DRY |
| R11 | 대화상자의 Esc 닫기가 세 번 복사돼 있다 | `menus.rs:135,227`, `shortcuts.rs:179` | DRY |
| R12 | 삭제 진입점이 둘이다. 단건 함수는 대기열 내부와 테스트에서만 직접 쓰인다 | `commands.rs:14`, `lifecycle.rs:287` | 일관성 |
| R13 | GTK 테스트 보조 함수가 파일마다 있고 고정 시간 대기에 의존해 부하에서 실패한다 | `drain`·`settle`·`until` 3곳. 관찰된 실패: `canvas/tests.rs` 터치패드·카드 크기·포커스 검사, `live_editor` 링크 카드 검사 | 테스트 신뢰성 |
| R14 | 모든 GTK 시나리오가 테스트 하나 안에서 돌아 첫 실패가 뒤를 가리고 부분 실행이 안 된다 | `gtk_tests.rs` | 테스트 구조 |
| R15 | 스모크 스크립트가 같은 보조 함수를 복사하고 좌표를 숫자로 적는다 | `tools/behavior/*.py` 5개의 `wait_window`·`key`·`click` | DRY, 변경 용이성 |

## 목표 구조

```text
src/app/editor/
  mod.rs            # 편집 창 조립
  live_editor/
    mod.rs          # attach·configure, 갱신 흐름(schedule·refresh)
    tags.rs         # EditorTags 구조체, 테마 색 반영
    layout.rs       # 내어쓰기 태그·폭 보정
    markers.rs      # 체크박스·기호 위젯 풀과 배치
    input.rs        # 제스처·포인터·키 연결
    tests.rs
  link_preview/
    mod.rs          # 상태 전이·타이머
    card.rs         # 카드 위젯·내용 채우기
  list_edit.rs
  formatting.rs

src/presentation/markdown/
  mod.rs            # 공개 타입과 parse
  inline.rs         # 서식·inline 링크
  list.rs           # line_mark·scan_items
  links.rs          # 일반 주소 인식·link_info
  summary.rs        # summary·fit_preview
```

경로 이동 후에도 기능 간 직접 참조 금지는 그대로다. `architecture_tests`를 함께 통과해야 한다.

## 단계

각 단계는 독립 커밋이다. 파일 이동과 내용 수정을 섞지 않아 `git log --follow`로 추적할 수 있게 한다. 끝날 때마다 변경 범위에 맞는 검사만 실행한다. 공통 검사와 스모크 명령은 OPS-003을 따른다.

### R-1. 테스트 기반 먼저 (R13, R14)

- `drain`·`settle`·`until`을 `src/app/test_support.rs`(`#[cfg(test)]`) 하나로 모은다. 위젯 탐색 보조 함수도 옮긴다.
- 고정 대기를 조건 대기로 바꾼다. 위 "관찰된 실패" 지점을 먼저 처리한다.
- 창 관리자 없는 Xvfb에서는 포커스가 흔들린다. `has_focus` 대신 `is_focus`로 검사한다.
- 시나리오를 나눌 수 있는지 먼저 확인한다. GTK는 한 스레드만 허용하므로 별도 프로세스 실행 또는 시나리오별 격리·실패 보고 중 하나를 고른다.
- 완료: 동작 변화 없음, 테스트 이름 유지 또는 대응 표 제시, 해당 시나리오를 단독 실행할 수 있다.

### R-2. 파일 분리 (R1, R4)

- `markdown.rs`와 `live_editor.rs`를 목표 구조로 나눈다. 테스트 모듈을 맨 뒤 또는 별도 파일로 옮겨 `cargo clippy --all-targets`의 "items after test module"을 없앤다.
- 경로 이동만 한다. 함수 본문은 바꾸지 않는다.
- 완료: 외부 호출부는 `use` 경로 외 변화 없음.

### R-3. 편집기 상태 묶기 (R2, R3, R5)

- `create_tags`의 튜플을 `EditorTags`로 바꾼다.
- `LiveEditor` 필드를 `EditorTags`, 위젯 풀, 입력 상태(`composing`·`edited`·`caret_moved`·`commit_newline`), 포인터 상태로 묶는다.
- `Document::item_at(offset)`을 추가하고 호출부를 바꾼다. 원문 조각 읽기는 보조 함수 하나로 모은다.
- 주의: `refresh` 안에서 `document`를 빌린 채 버퍼를 바꾸면 신호 재진입으로 이중 대여 패닉이 날 수 있다. 대여 범위를 넓히지 않는다.
- 완료: `fmt_001`·`fmt_003` 시나리오 무변경 통과.

### R-4. 링크 카드 상태 모델 (R6, R7) — 내부 인터페이스, 사용자 확인 후

- 상태를 하나의 열거형으로 바꾼다.

```rust
enum CardState {
    Idle,
    Pending { link: Link, source: Source, anchor: gdk::Rectangle },
    Shown { link: Link, source: Source },
    Leaving,
}
```

- `hover(None, 빈 사각형)`·`caret(None, 빈 사각형)`을 `pointer_left()`·`caret_left()`로 대체한다. 연속 호출 처리는 전이 함수 하나로 모은다.
- 카드 위젯 구성은 `card.rs`로 옮긴다.
- 유지할 제약: 아래 "유지할 GTK 제약" 전체.
- 완료: 지연·유예·Esc·입력·스크롤·창 닫기 동작이 FMT-003 그대로다.

### R-5. 표식 모양 단일 원본 (R8)

- 기호 목록을 `presentation`의 한 곳으로 옮긴다. 편집기와 카드가 같은 값을 쓴다.
- 체크 모양의 공통 색·모서리 값을 CSS 토큰으로 묶는다. 크기 차이(편집기 13px, 카드 12px)는 유지한다.
- 화면 변경이 아니므로 수정 전후 캡처가 같아야 한다.

### R-6. 캔버스·공통 정리 (R9, R10, R11, R12) — 내부 인터페이스, 사용자 확인 후

- `selected_ids()`·`selected_views()`를 두고 반복 코드를 없앤다.
- 위젯 탐색을 `shared`의 `find_named` 하나로 모은다. `keyboard::connect_menu`는 이름 탐색 대신 패널이 메뉴 버튼 참조를 보관하는 안을 우선 검토한다.
- `shared`에 `close_on_escape(window)`를 두고 대화상자 세 곳을 바꾼다.
- 삭제 진입점을 `delete_notes` 하나로 줄이고 단건 함수는 `commands` 내부로 숨긴다.

### R-7. 스모크 공통화 (R15)

- `tools/behavior/lsm_driver.py`로 창 찾기·키·클릭·합성 데이터·URI 핸들러 구성을 모은다.
- 스크립트의 좌표에 이름을 붙이고 한곳에서 계산한다.
- 완료: 기존 스크립트의 검사 항목이 줄지 않는다.

## 권장 순서

```text
R-1 → R-2 → R-3 → R-4
             └→ R-5 → R-6 → R-7
```

R-2는 다른 작업과 충돌하기 쉬우므로 짧게 끝낸다. R-5 이후는 병행해도 파일이 거의 겹치지 않는다.

## 유지할 GTK 제약

주석을 지우거나 "단순화"하면 회귀하는 지점이다. 관련 코드를 옮길 때 주석과 함께 옮긴다.

- 줄 간격은 `pixels_above_lines`만 쓴다. `pixels_below_lines`를 쓰면 숨긴 글자가 있는 줄 아래 간격에서 GTK가 중단된다(`gtk_text_iter_set_visible_line_index`).
- 링크 카드는 편집기 unmap 때 떼어 낸다. `GtkTextView`는 파괴될 때 자기 것이 아닌 자식이 있으면 끝나지 않는 반복에 빠진다.
- 링크 카드는 닫을 때마다 폐기하고 새로 만든다. 다시 띄운 팝오버 표면은 X11에서 포인터 진입을 받지 못한다.
- 표식 글자의 투명 색은 알파 0이 아니라 아주 작은 값이다. Pango는 알파 0을 지정 안 함으로 읽는다.
- 숨긴 글자가 있는 줄은 `iter_location`의 가로 값을 믿지 않는다. 세로 값만 쓴다.
- 포인터 아래 글자는 `iter_at_position`으로 구한다. `iter_at_location`은 가까운 경계로 반올림한다.
- 제자리 포인터 이동은 카드 예약으로 치지 않는다. GTK가 표면 변경 후 이동 이벤트를 합성한다.
- 스크롤 조절기는 현재 값과 이후 교체를 모두 연결한다. 편집기는 스크롤 뷰에 들어간 뒤 `attach`된다.
- 빈 항목의 표식은 숨기지 않는다. 줄 전체가 숨겨지면 줄이 접힌다.

## 하지 않는 것

- 성능 개선. 목록 항목 위젯 배치의 뷰포트 제한도 이번 범위가 아니다. 실제 사용에서 문제가 확인되면 별도로 결정한다.
- 동작·시각 디자인 변경과 새 기능.
- `src/persistence`와 공개 계약 변경. 삭제 대기열을 백엔드 일괄 삭제로 옮기는 일도 포함한다.
- 프레임워크, 이벤트 버스, 트레이트 추상화 도입.
- 카드 체크박스 직접 토글. 원본 편집기 버퍼 동기화가 필요한 기능 변경이다.

## 위험

| 위험 | 대응 |
|---|---|
| 파일 이동으로 진행 중인 작업과 충돌 | R-2를 경로 변경 전용으로 짧게 처리 |
| 상태 묶기 중 `RefCell` 이중 대여 패닉 | 대여 범위를 함수 안으로 제한, `fmt_003` 입력 시나리오로 확인 |
| 링크 카드 전이 회귀 | 전이 표를 먼저 쓰고 `fmt_003_link_preview_lifecycle`과 스모크로 확인 |
| 좌표 기반 스모크가 깨짐 | 이 계획은 레이아웃을 바꾸지 않는다. 깨지면 회귀로 본다 |

## 미확인으로 남은 것

- 실제 Wayland·한글 IME의 조합 중 Enter 동작. 현재는 조합 신호 재현 검사만 있다. 리팩터링 뒤에도 같은 수준임을 구분해 보고한다.
