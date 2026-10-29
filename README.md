# Linux Sticky Memo

Ubuntu용 Rust/GTK 스티키 메모. 목록 검색, 독립 편집 창, 색상 변경, 자동 저장을 제공한다.

- 빠른 실행: GTK 개발 패키지·SVG 로더 설치 후 `cargo run --release --locked`.
- 설치·업데이트: OPS-001.
- 백업·복원·저장 오류: OPS-002.
- 개발 검증·테스트 환경: OPS-003.

## 탐색

- `src`: app(창·위젯·상태·조율), persistence(저장·경로), presentation(테마·아이콘·스타일).
- `docs`: architecture·decisions·specs·operations·verification. 관심사·고정 ID로 검색.
- `tools`: behavior·quality 검증 도구.
- `integration`: 실행기·설치·앱 메뉴 자산.
