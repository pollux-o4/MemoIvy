# OPS-001 · 설치·업데이트

- 전제: Ubuntu, Rust, GTK 4.6 이상 개발 패키지, SVG 로더, C 컴파일러.
  - Rust 선택은 `.mise.toml`, 크레이트·GTK 기능은 Cargo manifest가 원본이다.
- 시스템 의존성
  - `sudo apt install build-essential pkg-config libgtk-4-dev librsvg2-common zenity`
- 실행
  - 빌드: `cargo build --release --locked`
  - 직접 실행: `./target/release/linux-sticky-memo`
  - 설치·업데이트: `./integration/install.sh`
  - 설치기는 현재 HOME과 `바탕화면/서비스 실행 파일` 폴더를 사용한다. 폴더가 없으면 먼저 만든다.
- 확인: 폴더의 Linux Sticky Memo 또는 앱 메뉴에서 목록·편집 창을 연다. 기존 프로세스가 열려 있으면 정상 종료 후 다시 실행한다.
- 실패 대응
  - 실행 로그: `~/.local/share/linux-sticky-memo-launch.log`.
  - SVG 로더·GTK 버전·설치 대상은 `launcher`, `install` 검색으로 확인한다.
- 제거: 설치기의 출력 대상인 앱·실행기·메뉴·아이콘·폴더 링크를 제거한다. 메모 데이터의 보관·복원은 OPS-002를 따른다.
