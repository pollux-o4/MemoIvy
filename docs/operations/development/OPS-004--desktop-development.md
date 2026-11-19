# OPS-004 · Electron 화면 개발

검색 키: desktop / Storybook / SQLite / migration / engine_transport.

## 실행

Node와 Rust는 프로젝트 mise 설정을 사용한다. `npm ci`로 잠긴 의존성을 설치한다.

| 작업 | 명령 |
|---|---|
| Rust 엔진·React·Electron 함께 개발 | `mise run dev` |
| 표본 데이터(카드·연결·프로젝트 경우의 수)로 개발 | `mise run dev:sample` (초기화: `mise run dev:sample:reset`, 정의: `tools/dev-data/sample_notes.cjs`) |
| 화면 상태별 검수 | `mise run storybook` |
| 브라우저 화면 미리보기 | `npm run dev:web` |
| 화면 타입·프로덕션 번들 확인 | `npm run typecheck`, `npm run build:web` |
| 엔진 관련 검증 | `mise run test:engine` |
| 데스크톱 통신 관련 검증 | `mise run test:desktop` |
| 화면 순수 로직 단위 테스트 | `mise run test:web` |
| 스토리 상호작용·접근성(axe) | `mise run test:stories` (현재 코드로 정적 빌드 후 검사. 띄워 둔 dev 서버로는 `npm run test:stories:live`) |
| 캔버스 조작 부드러움(휠·휠클릭 이동·확대·카드 끌기·가지 초점) | `mise run test:perf` (카드 200장·간선 400개 스토리, 프레임 평균 ≤10ms·p95 ≤16ms·이동 튐 ≤2.5배·선택 클릭 프레임 ≤33ms. 캔버스 렌더·CSS 변경 시) |
| 캔버스 메모리 회복 | `mise run test:memory` (화면 전환·본문 펼침·끌기 50회 뒤 GC 힙 변화 ≤10%. 리스너·캐시·관찰자 변경 시) |
| 실제 창 작성·동기화·재시작 복원 | `mise run test:e2e` (합성 데이터 폴더, xvfb) |
| 린트·미사용 코드·모듈 경계·중복·미사용 crate | `mise run check:web` |
| 화면 상태 번들 | `npm run build:storybook` |
| 현재 운영체제 설치 패키지 빌드 | `npm run desktop:build` |
| 설치 없이 배포 디렉터리 생성 | `npm run desktop:dir` |
| 기존 GTK 화면 비교 | `mise run dev:legacy` |

검증은 변경 영역만 실행한다. 모듈 경계 규칙(shared는 features를 모르고 기능끼리는 shared로만 공유)은 `.dependency-cruiser.cjs`가 정한다. 브라우저 미리보기의 가짜 데이터는 실제 메모 저장소와 다르며 화면에 표시한다. 실제 저장은 Electron에서 확인한다. Storybook은 6006, Vite는 5173을 사용한다.

## 데이터

- Linux는 절대 경로 XDG_DATA_HOME 또는 `~/.local/share` 아래 memoivy, Windows·macOS는 운영체제 앱 데이터 아래 MemoIvy를 사용한다.
- `MEMOIVY_DATA_DIR`에 절대 경로를 주면 합성 데이터 전용 폴더를 사용할 수 있다. 개발·검수는 사용자 원본과 분리한다.
- SQLite 원본은 `memoivy.sqlite3`다. 실행 중인 데이터베이스 파일만 복사하지 말고 앱을 완전히 종료한 뒤 데이터 폴더를 백업한다.
- 이전 `notes.json`은 원본과 백업을 남겨 가져온다. 자동으로 찾지 못한 Snap 데이터는 화면의 이전 데이터 가져오기로 JSON을 선택한다.
- 기존 GTK 앱과 엔진은 `.memoivy.lock`을 공유한다. SQLite로 가져온 폴더를 기존 GTK 앱으로 다시 편집하지 않는다.

## 관찰 범위

화면 이관은 UI-REF-001의 GTK 외형을 유지한다. Storybook의 `Reference/GTK Fidelity`에서 원본 PNG와 같은 데이터·테마·치수의 이관 화면을 비교한다. 원본 자산 재캡처는 사용자가 명시적으로 요청했을 때만 `mise run test:gtk-reference`로 실행한다.

Linux에서 실행 화면과 저장 복원을 확인한다. Windows·macOS는 해당 환경에서 실제 확인한 범위만 기록한다. Mac이 없다는 이유로 Linux 개발 완료를 막거나 운영체제별 자동 검사 체계를 새 필수 과업으로 만들지 않는다.
