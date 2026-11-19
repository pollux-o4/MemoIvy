# OPS-001 · 설치·업데이트

## 기존 Snap 배포본

- Linux에서 `sudo snap install memoivy`로 설치하고 `snap run memoivy`로 실행한다. 수동 갱신 전 앱을 종료하고 `sudo snap refresh memoivy`를 실행한다.
- 로컬 `.snap` 설치본의 “스토어에서 알 수 없음” 오류는 앱을 닫고 `sudo snap refresh memoivy --amend --channel=stable`로 배포 채널을 전환한다.
- 이전 데이터 저장 위치와 가져오기는 OPS-002를 따른다.
- 기존 GTK Snap을 재현할 때는 별도 체크아웃에서 `git worktree add --detach ../memoivy-gtk-snap 599324e`를 실행한다. 그곳에서 당시 README와 Snapcraft 빌드 환경을 사용해 `mise run package:snap`으로 패키지를 만든다.

## Electron 개발판

- Linux 데스크톱에서 mise로 Node·Rust와 선언된 시스템 도구를 준비하고 `npm ci`를 실행한다. Electron 실행에는 배포판의 그래픽·오디오 런타임을 사용한다. 개발 실행은 OPS-004를 따른다.
- 앱을 종료한 뒤 `mise run install`을 실행하면 Linux 배포 폴더를 빌드해 사용자 홈의 앱 메뉴와 바탕화면 실행기를 갱신한다. 설치 후 앱 메뉴에서 목록·편집 창을 열어 확인한다.
- 실패 시 `~/.local/share/memoivy-launch.log`와 설치기 출력을 확인한다. 앱·실행기 제거와 별개로 데이터를 보관하려면 OPS-002를 따른다.
