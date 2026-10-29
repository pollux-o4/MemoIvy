# Linux Sticky Memo

Ubuntu용 GTK 4 스티키 메모입니다. 목록에서 메모를 만들고 검색하거나 여러 메모를 각각 편집할 수 있습니다. 내용과 색상은 자동 저장됩니다. 창을 닫아도 메모는 삭제되지 않습니다.

## 빌드와 실행

Ubuntu에서 Rust와 GTK 4.6 이상 개발 패키지 및 SVG 이미지 로더를 설치합니다.

```sh
sudo apt update
sudo apt install build-essential pkg-config libgtk-4-dev librsvg2-common
# Rust 설치: https://rustup.rs/
# mise를 쓰는 경우 프로젝트의 .mise.toml이 Rust 1.99.1을 선택합니다.
cargo build --release
./target/release/linux-sticky-memo
```

## 사용자 설치

```sh
./integration/install.sh
```

설치 스크립트는 현재 사용자의 홈과 바탕화면의 `서비스 실행 파일` 폴더를 사용합니다. 업데이트할 때 release 빌드 후 설치 스크립트를 다시 실행하세요. 실행 오류는 `~/.local/share/linux-sticky-memo-launch.log`에 기록됩니다. 삭제할 때는 설치된 실행 파일, 실행기, 아이콘, 앱 메뉴 항목과 폴더 링크를 지우세요.

메모는 `$XDG_DATA_HOME/linux-sticky-memo/notes.json`에 저장되며 파일은 소유자만 읽고 쓸 수 있습니다. `XDG_DATA_HOME`이 없거나 절대 경로가 아니면 `~/.local/share/linux-sticky-memo/notes.json`을 사용합니다. 백업할 때 이 파일을 복사하세요. 데이터를 읽을 수 없으면 원본을 보존하고 오류를 표시합니다. 저장 오류가 발생하면 저장 위치와 권한을 확인하세요. 입력은 잠시 멈춘 뒤 또는 계속 입력 중에는 주기적으로 백그라운드에 저장합니다. 정상적으로 창을 닫을 때 저장 완료를 기다리며, 강제 종료나 전원 차단 시 아직 저장되지 않은 변경은 유실될 수 있습니다. 저장이 느리거나 실패하면 저장 대기 시간이 길어질 수 있습니다.

## 개발 검증

`cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked -- -D warnings`, `cargo build --locked`로 확인합니다. 창 관리자가 있는 가상 X11 화면에서 메모 생성, 완성형 한글과 이모지 붙여넣기, 색상 변경, 여러 창, 검색, 닫기, 재실행 복원과 삭제를 확인했습니다. 한글 입력기의 조합 입력은 검증하지 않았습니다.

개발 패키지를 시스템에 설치할 수 없는 이 작업 환경에서는 패키지를 `/tmp/lsm-deps/root`에 추출하고 런타임 라이브러리 링크를 `/tmp/lsm-libs`에 만든 뒤, 검증 명령 앞에 `PKG_CONFIG_PATH=/tmp/lsm-deps/root/usr/lib/x86_64-linux-gnu/pkgconfig LIBRARY_PATH=/tmp/lsm-libs`를 붙였습니다. 일반 Ubuntu 설치에서는 위 `apt install`만 필요합니다.

2026-10-29 격리 XDG·Xvfb/Openbox 검증에서는 마지막 입력 직후 닫기와 저장 실패 후 재시도를 확인했습니다. 삭제 저장 실패 후 다른 메모의 편집과 원본 메모가 유지되는 것을 확인했지만, 편집이 쓰기 작업 진행 중 발생했는지는 확인하지 못했습니다. `artifacts/ui-review/performance-probe.txt`의 수치는 합성 probe의 결과이며 실제 편집 프레임이나 전체 GTK 입력 경로의 지연 측정은 아닙니다. GNOME/Wayland, 화면 읽기 프로그램, 입력기 조합 입력은 이 검증에 포함되지 않았습니다.

내장 SVG 디코딩 검증은 이미지 로더가 동작하는 세션이 필요합니다. 이 환경의 Glycin 로더는 일반 sandbox에서 D-Bus 소켓 권한 오류가 발생해, `dbus-run-session -- cargo test --locked`를 sandbox 밖의 격리 세션에서 실행했습니다.

## 코드 구조

진입점은 `src/main.rs`, 앱 조립과 비공개 상태는 `src/app/mod.rs`입니다. `app/chrome.rs`는 창 장식, `cards.rs`는 목록 카드, `library.rs`는 목록 창, `editor.rs`는 편집 창, `menus.rs`는 삭제 메뉴, `controller.rs`는 저장·닫기 조율을 담당합니다. `theme.rs`는 테마 판정, `paths.rs`는 데이터 경로, `icons.rs`는 SVG, `save_queue.rs`는 비동기 저장, `store.rs`는 데이터 검증·원자 저장을 담당합니다. 분석과 패턴 선택은 `REFACTORING.md`에 기록했습니다.


UI 회귀 검사는 `dbus-run-session -- xvfb-run -a python3 tools/ui_smoke.py`로 실행합니다. `xdotool`, `openbox`, Xvfb가 필요하며 별도 경로에 있다면 `LSM_XDOTOOL`, `LSM_OPENBOX`와 해당 라이브러리 경로를 지정합니다. 임시 데이터만 사용하며 생성·입력·즉시 닫기 저장·종료·재실행·메모 다시 열기를 확인합니다.
