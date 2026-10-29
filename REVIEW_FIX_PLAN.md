# 검토 결과와 Terra 수정 계획

## 확인한 내용
- src/main.rs, src/store.rs, Cargo.toml, README와 desktop 항목을 검토했다.
- 기존 cargo test --locked 4개와 cargo clippy --locked -- -D warnings 통과.
- 단일 사용자가 사용하는 로컬 앱 기준으로 생성/수정/삭제, 저장 실패, 창 닫기와 활성화 흐름을 추적했다.

## 수정 필요: 저장할 때 비공개 권한이 풀림
- 위치: src/store.rs save(), 임시 파일 생성 후 rename하는 부분.
- OpenOptions가 권한을 지정하지 않아 umask에 따라 파일을 생성한다. 현재 umask 0002에서는 신규 파일이 0664이며, 기존 파일을 0600으로 바꾼 후 저장해도 0664로 교체된다.
- /tmp/lsm-review-permissions.rs에서 실제 Store::save를 호출해 재현했다. 신규 저장 664, 기존 600 파일 재저장 후 664 출력 및 600 기대 assertion 실패.
- HOME이 비공개이면 경로 접근이 차단될 수 있지만, 공유 접근 가능한 XDG_DATA_HOME에서는 다른 사용자에게 메모가 노출된다. 기존 파일의 제한 권한도 자동 저장이 해제한다.

## 최소 구현 지시
1. Linux/Unix OpenOptionsExt::mode(0o600)로 임시 파일을 처음부터 소유자만 읽고 쓸 수 있게 만든다. 저장 완료 후 권한을 줄이는 방식은 중간 노출이 있으므로 사용하지 않는다.
2. 공유 XDG_DATA_HOME 전체의 권한을 변경하지 않는다. 앱 폴더 권한 변경은 필요한 경우에만 별도 판단하며 외부 경로를 손대지 않는다.
3. 기존 원자적 rename, file/directory sync 및 실패 시 임시 파일 정리를 유지한다.
4. src/store.rs에 신규 저장 권한과 0600 파일 재저장 권한을 검사하는 Unix 회귀 테스트를 추가한다. 권한 검사에 mode & 0o077 == 0을 사용하면 제한적인 umask도 수용한다. 프로세스 전역 umask를 테스트에서 바꾸지 않는다.
5. 테스트를 수정 전 먼저 실행해 실패를 확인한 후 수정한다. README에 메모 파일의 소유자 전용 접근을 짧게 설명한다.

## 완료 검증
- cargo fmt --check, cargo test --locked, cargo clippy --locked -- -D warnings, cargo build --locked.
- /tmp/lsm-review-permissions 재현 harness도 다시 컴파일해 새 코드로 통과 확인.
- 빌드 환경: PKG_CONFIG_PATH=/tmp/lsm-deps/root/usr/lib/x86_64-linux-gnu/pkgconfig LIBRARY_PATH=/tmp/lsm-libs.
- 이번 수정은 저장 권한에 한정한다. 실제 GNOME/Wayland와 IME 조합 입력의 기존 미검증 상태를 완료했다고 주장하지 않는다.
- 발견했으나 범위 밖으로 미룬 구체적 문제는 사용자 shortcut 주석 규칙을 따른다. 위 권한 문제는 완료 전 해결한다.
