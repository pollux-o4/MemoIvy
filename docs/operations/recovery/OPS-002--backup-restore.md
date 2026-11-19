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
