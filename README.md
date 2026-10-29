# Linux Sticky Memo

Ubuntu용 Rust/GTK 스티키 메모. 목록 검색, 독립 편집 창, 색상 변경, 자동 저장을 제공한다.

Markdown 서식은 입력하는 화면에 실시간 반영된다. 커서·선택이 있는 줄은 문법을 보여준다. 체크는 클릭·Ctrl+Enter, 링크 열기는 Ctrl+클릭을 사용한다. 원문 보기·서식 도움말은 더보기 메뉴에 두고, 원문 전환은 Ctrl+Shift+M도 지원한다. 앱·서식 단축키 전체 안내는 F1로 연다.

목록·편집기의 **캔버스**에서 메모를 배치하고 다음 메모를 연결한다. 카드 본문은 원본 메모를 열고, 이동 핸들은 선택·드래그·방향키 배치를 지원한다. 완료 시 연결을 따라 일회성 신호를 보내며 배치·연결·완료 변경은 되돌릴 수 있다.

- 빠른 실행: GTK 개발 패키지·SVG 로더 설치 후 `cargo run --release --locked`.
- 설치·업데이트: OPS-001.
- 백업·복원·저장 오류: OPS-002.
- 개발 검증·테스트 환경: OPS-003.

## 탐색

- `src`: app(창·위젯·상태·조율), persistence(저장·경로), presentation(테마·아이콘·스타일).
- `docs`: architecture·decisions·specs·operations·verification. 관심사·고정 ID로 검색.
- `tools`: behavior·quality 검증 도구.
- `integration`: 실행기·설치·앱 메뉴 자산.
