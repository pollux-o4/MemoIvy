# C4 · 데스크톱 통합 코드 탐색

| 관심사 | 탐색 키 |
|---|---|
| 창 수명과 엔진 실행 | `desktop/main.cjs`의 `createWindow`·`launchEngine`·`doQuit` |
| 앱 경계 | `preload.cjs`, `ipc_validation.cjs`, `EngineTransport`, `external_links.cjs` |
| 화면 자산 | `frontend/shared/assets/icons`, `Icon`, 화면별 CSS와 공통 `tokens.css` |
| Linux 설치 | `integration/install.sh`, `launcher.c`, 데스크톱 파일 |

설치·갱신은 OPS-001, 창과 아이콘은 UI-001·UI-002를 따른다.
