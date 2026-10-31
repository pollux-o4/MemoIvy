# C4 Code · 표현·실행 통합 탐색

## Markdown 해석

`presentation::markdown` 루트가 공개 타입·함수와 공통 기호 `BULLETS`를 모은다.

| 관심사 | 탐색 키 |
|---|---|
| 서식·원문 범위 | `src/presentation/markdown/syntax` inline의 `parse`·`plain_line`, `Document::item_at` |
| 목록 인식 | syntax list의 `line_mark`·`scan_items` |
| 링크 판별·목적지 표시 | syntax links의 `safe_link`·`link_info` |
| 카드 요약 | summary의 `summary`·`fit_preview` |

## 시각 기반

- CSS 설치·테마 구독: presentation의 `install`, theme의 `is_dark`·`WATCHED`.
- SVG 렌더링·캐시: icons의 `image`·`illustration`·`cached`.
- 표식 공통 외관: styles/views의 `.md-checkbox check`·`.line-marker.task`; 색상 원본은 styles/themes의 `tokens-light`·`tokens-dark`.

## 데스크톱 통합

integration의 launcher `main`은 앱 실행·실패 로그를 맡는다. install 스크립트가 실행기·아이콘·앱 메뉴·바탕화면 링크를 배치하고, desktop 템플릿의 `Exec`·`Icon`을 설치 경로로 확정한다.

설치·갱신·진단: OPS-001. 아이콘 계약: UI-001, UI-002.
