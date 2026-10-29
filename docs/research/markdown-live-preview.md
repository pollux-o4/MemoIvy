# Markdown Live Preview 조사 · 2026-10-29

## 확인한 동작

- Obsidian의 기본 편집 모드는 Live Preview다. 본문 안에서 서식을 표시하고 대부분의 Markdown 문법을 숨기며, 커서가 서식 영역에 들어가면 편집할 원문을 드러낸다. 읽기 화면과 원문 모드도 별도로 제공한다. 따라서 편집/미리보기 버튼으로 화면 전체를 교체하는 현재 LSM보다, 같은 편집 화면에서 커서 주변만 원문으로 드러내는 모델이 요청에 가깝다. [Obsidian 편집 모드](https://obsidian.md/help/edit-and-read)
- 체크 목록은 Markdown 원문이며 읽기 화면에서도 체크박스를 선택해 상태를 바꿀 수 있다. 단순한 체크 글리프 표시는 이 동작을 제공하지 않는다. [Obsidian 기본 서식](https://obsidian.md/help/syntax)
- 링크 제스처는 모드별로 구분된다. 현재 탭 설명은 일반 선택을 이동, Ctrl 선택을 새 탭으로 안내한다. 별도 입문 문서는 Ctrl+클릭 이동을 설명한다. 이를 “현재 Obsidian 전체에서 반드시 Ctrl+클릭”으로 일반화하지 않는다. [탭](https://obsidian.md/help/tabs), [링크 입문](https://obsidian.md/help/link-notes)

## LSM 구현 권고 · 근거에서 도출한 설계

- `Note.text`와 단일 `TextBuffer`의 Markdown을 원본으로 유지하고, 렌더링은 태그·표시 상태로만 바꾼다. 재렌더에 `set_text()`를 쓰면 현재 내용을 삭제하고 교체하며 undo 스택에서도 되돌릴 수 없는 동작으로 처리된다. 체크 토글 같은 실제 편집은 `begin_user_action()`/`end_user_action()`으로 한 번의 되돌리기로 묶는다. [GTK set_text](https://docs.gtk.org/gtk4/method.TextBuffer.set_text.html), [사용자 동작](https://docs.gtk.org/gtk4/method.TextBuffer.begin_user_action.html)
- 커서와 선택이 닿는 서식의 문법을 보이게 하고, 다른 부분에 제목·강조·목록을 적용한다. `TextTag.invisible`로 구문을 숨길 수 있지만 GTK는 숨긴 구간의 프로그램적 탐색 문제를 경고한다. 커서 경계·선택·삭제 검증을 통과해야 채택한다. 원문 모드는 복잡한 선택·수정의 복구 경로로 유지한다. [GTK invisible](https://docs.gtk.org/gtk4/property.TextTag.invisible.html)
- 저장·검색·원문 복사에는 숨긴 문자를 포함한다. `get_text(..., false)`는 숨긴 문법을 제외하므로 Markdown 손실을 일으킬 수 있다. 파서의 원문 범위와 GTK의 문자 오프셋을 구분하고, 한글·이모지 앞뒤의 범위 변환을 검증한다. [GTK get_text](https://docs.gtk.org/gtk4/method.TextBuffer.get_text.html), [GTK 문자 오프셋](https://docs.gtk.org/gtk4/method.TextBuffer.get_iter_at_offset.html), [pulldown-cmark 원문 범위](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.OffsetIter.html)
- 한글 조합은 아직 버퍼에 확정되지 않은 preedit 상태다. 조합 중 현재 줄의 숨김·레이아웃 변경을 보류하고, 확정 후 갱신하는 방식을 권한다. 이는 GTK의 신호 의미로부터 도출한 구현 원칙이며 GTK가 특정 편집기 정책을 강제하는 것은 아니다. [GTK preedit-changed](https://docs.gtk.org/gtk4/signal.TextView.preedit-changed.html)
- 편집 화면의 일반 클릭은 커서·선택에 사용하고 Ctrl+클릭 또는 명시적 동작으로 링크를 연다. 이는 스티키 메모 편집 방해를 줄이기 위한 LSM 선택이다. 실행 가능한 스킴을 제한하고 셸 명령을 만들지 않는다. 체크박스 클릭은 `[ ]`/`[x]` 한 문자만 바꾸며 저장·undo를 기존 경로에 연결한다.
- 입력 이벤트마다 전체 위젯·전체 원문을 재생성하지 않는다. 한 프레임 또는 짧은 idle로 변경을 병합하고, 파싱 결과를 캐시해 커서 이동에는 표시 범위만 갱신한다. CodeMirror 공식 예제도 viewport 중심의 decoration, 위젯 재사용, 범위를 매핑하는 갱신과 atomic cursor 범위를 사용한다. 이 원칙을 GTK에 적용하는 것이지 CodeMirror의 브라우저 구현을 이식하는 것은 아니다. [CodeMirror decorations](https://codemirror.net/examples/decoration/)

## 적대적 검증 기준

- 동일 화면: 문법 입력→서식 완성→커서 이탈→문법 숨김→클릭/화살표 재진입→원문 복원이 즉시 대응한다. 선택 중인 구문은 숨기지 않는다.
- 데이터: 한글·이모지·중첩 강조·이스케이프·미완성 링크·중첩 목록에서 원문, 커서, 선택 범위를 보존한다. Ctrl+A→복사→붙여넣기로 원문이 일치한다.
- 편집: 숨긴 구간 경계의 Backspace/Delete/Home/End, 줄 합치기, 여러 줄 선택, undo/redo, 체크 토글 한 번의 undo를 확인한다. 실제 IME 조합과 단순 Unicode 붙여넣기는 다른 검증으로 기록한다.
- 회귀: SAVE-001의 마지막 입력 직후 닫기·재열기, SAVE-002 실패·재시도, PERF-001의 10KiB 연속 입력을 확인한다. 저장은 표시 상태에 영향받지 않아야 한다.

## 조사 한계

- Obsidian 실행본을 조작해 비교한 결과가 아니라 1차 공식 문서 조사다. 버전별 세부 선택 경계·애니메이션·외부 링크 제스처의 완전 일치를 주장하지 않는다.
- CodeMirror의 웹 페이지 직접 열기는 403이었으나 검색 서비스가 제공한 공식 예제 본문을 확인했다. GTK 문서는 최신 버전이므로 구현 시 프로젝트의 GTK 4.6 API 하한을 별도로 확인한다.

## FMT-001 구현·적대 검토 결과

- 기본 편집 화면을 단일 TextBuffer의 실시간 서식으로 변경했다. 커서·선택 줄은 문법을 드러낸다. 사용자 피드백에 따라 상단 도구 줄을 제거하고 원문·도움말은 더보기 메뉴로 옮겼다. 원문 단축키는 Ctrl+Shift+M이다.
- 수정 루프: UTF-8 바이트/문자 범위와 인접 줄 노출 경계 검증 → 원문 복사·잘라내기를 idle 전에도 보존 → 네이티브 체크박스의 원문 괄호 겹침·배치 지연 수정 → 전체 태그 교체를 범위 차이 갱신으로 변경했다. GTK overlay 제거 API의 제약은 창 수명 동안 체크 위젯을 재사용하는 방식으로 처리하고 관련 shortcut을 코드에 기록했다.
- 검증 환경: debug GTK4, 임시 GTK 개발 경로, 격리 D-Bus·Xvfb·Openbox와 임시 XDG 데이터. `mise run check`의 14개 테스트·clippy, build, SAVE-001 기본 UI smoke, FMT-001 실제 앱 조작을 통과했다. 밝은/어두운 테마 화면을 직접 확인했다.
- 실제 앱 조작: 새 Markdown 입력, 커서 재진입, 드래그·전체 선택·원문 복사, 줄 경계 Backspace/Delete, undo, 체크 클릭·Ctrl+Enter, 너비 변경, 원문 전환, 즉시 닫기·재열기. Ctrl+클릭은 임시 OS URI 핸들러가 전달 URL을 기록하며 외부 사이트 접속 없이 검증했다.
- 성능: 약 10KiB·230개 서식 줄의 실제 GTK 위젯 입력+idle 갱신 100회에서 개선 후 p95 7.042ms를 관측했다. 이는 debug 합성 위젯 경로이며 실제 앱 전체 입력→프레임 수치가 아니다. 기존 PERF-001의 저장/입력 경로 기준과 구분한다.
- 한계: 조합 신호 시작·종료와 조합 중 레이아웃 보류는 위젯 테스트로 검증했다. 격리 IBus의 실제 한글 엔진 전환은 반복 timeout으로 완료하지 못해 실제 IME 입력 성공을 주장하지 않는다. GTK 4.6 하한의 API 범위로 구현했으나 이번 런타임 검증은 호스트 GTK 버전이다. 지원 범위 밖 참조 링크·이미지·코드블록의 완전한 렌더링은 원문으로 남긴다.
- 배포 확인: `mise run install` 완료 후 설치된 릴리스 실행파일에도 동일 FMT-001 UI 조작 검사를 수행해 통과했다. 빌드 파일과 설치 파일의 SHA-256이 일치했다. 캡처는 `/tmp/lsm-live-installed`에 남겼다.
