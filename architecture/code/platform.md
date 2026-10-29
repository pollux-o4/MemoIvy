# C4 · 시각 기반·실행 통합 코드 라우팅

[맵 입구](../README.md) · [UI 컴포넌트](../03-ui-components.md) · [컨테이너](../02-containers.md)

| 관심사 | 소스 | 시작 지점 |
|---|---|---|
| 단일 테마 판정·변경 구독 | [theme.rs](../../src/theme.rs) | `is_dark`, `watch` |
| 내장 SVG·색/배율별 렌더링 | [icons.rs](../../src/icons.rs) | `image`, `render`, SVG decode 테스트 |
| 아이콘 원본 | [icons/](../../src/icons/) | close/minimize/maximize/restore/palette/more/add SVG |
| 위젯 외관 | [style.css](../../src/style.css) | `.lsm-window`, `.header`, `.window-action`, `.card`, `.memo` |
| native 실행기·오류 로그 | [launcher.c](../../integration/launcher.c) | `main` |
| 사용자 설치·고정 경로·아이콘 등록 | [install.sh](../../integration/install.sh) | 앱/실행기/메뉴/SVG 설치 |
| 앱 메뉴 템플릿 | [desktop 항목](../../linux-sticky-memo.desktop) | `Exec`, `Icon` 및 설치 시 HOME 치환 |
| 앱 아이콘 원본 | [앱 SVG](../../integration/linux-sticky-memo.svg) | 런처용 아이콘; UI 내부 SVG와 용도가 다름 |

공유 정책을 변경할 때 테마 판정은 `theme.rs`, 색상 허용값은 [store::COLORS](../../src/store.rs), 실제 설치 위치는 `install.sh`를 먼저 확인한다. SVG를 바꿀 때는 `icons.rs`의 1×/2× decode 테스트도 확인한다.
