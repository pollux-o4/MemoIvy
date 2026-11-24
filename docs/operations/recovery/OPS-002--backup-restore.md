# OPS-002 · 백업·복원·저장 오류

## Electron 개발판

- 저장 원본은 데이터 폴더의 `memoivy.sqlite3`다. Linux는 절대 경로 `XDG_DATA_HOME` 또는 `~/.local/share` 아래 `memoivy`를 사용한다. 실제 경로는 `dataDirectory`에서 정한다.
- 백업은 앱을 정상 종료한 뒤 데이터 폴더 전체를 복사한다. 첨부 파일 데이터도 SQLite에 저장한다.
- 복원은 앱을 종료하고 현재 폴더를 별도 보관한 뒤 백업 폴더를 제자리에 복사한다. 재실행해 메모 수·본문·첨부를 확인한다. 실패하면 앱을 닫고 보관한 폴더로 되돌린다.
- 저장이 실패하면 편집 창을 유지하고 디스크 공간과 권한을 확인한 뒤 다시 시도한다.

## 이전 설치본 데이터 가져오기

- 기존 일반 설치본은 `XDG_DATA_HOME` 또는 `~/.local/share` 아래 `memoivy`에서 `notes.json`을 찾는다. 초기 설치본은 같은 위치의 `linux-sticky-memo`를 사용한다.
- 기존 Snap은 `snap run --shell memoivy`로 연 셸에서 `printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/memoivy"`를 실행해 데이터 폴더를 확인한다. 셸은 `exit`로 닫는다. 기존 설치본끼리의 복원 절차는 `599324e`의 OPS-002에 보존했다.
- 이전 설치본을 종료하고 `notes.json`과 같은 폴더의 `assets/` 및 JSON 색인 파일을 함께 보관한다.
- 처음 실행할 때 원본을 탐색한다. 자동으로 찾지 못하면 설정 → 기록 → 이전 메모의 **가져오기**에서 `notes.json`을 선택한다. 가져오기에는 변경 이력이 없는 빈 저장소가 필요하다.
- 가져오기는 원본 폴더 잠금을 확인하고 `notes.json.memoivy-backup`을 원본 옆에 저장한다. 원본·백업·첨부 보존의 검증은 DATA-001·003을 따른다.

## Electron Snap 전환과 복구

- 기존 GTK 배포본은 리비전별 `$SNAP_USER_DATA/.local/share/memoivy`에 JSON과 첨부를 저장한다. Electron Snap은 갱신 때 복사된 리비전의 원본을 가져오고 SQLite를 `$SNAP_USER_COMMON/memoivy`에 저장한다.
- 앱을 닫고 리비전별 원본, 공유 SQLite 폴더와 첨부를 백업한다. 리비전을 되돌리면 GTK는 기존 JSON을 읽는다. Electron에서 새로 작성한 메모는 공유 SQLite에 남으므로, Electron 후보를 다시 설치한 뒤 확인한다.
- 되돌린 GTK 앱이 SVG 로더 오류로 종료되면 앱을 닫고, 되돌린 리비전의 `~/snap/memoivy/<리비전>/.last_revision`을 다른 이름으로 보관한다. 다시 실행하면 GNOME 런처가 현재 리비전의 캐시를 재생성한다. JSON·첨부·공유 SQLite는 그대로 둔다.
