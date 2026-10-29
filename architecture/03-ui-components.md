# C3 · UI 컴포넌트

```mermaid
flowchart TB
    lifecycle["앱 조립 · run"] --> library["목록 창 · library"]
    library --> cards["카드 표시 · cards"]
    cards -->|열기| editor["독립 메모 · editor"]
    cards --> menus["삭제 확인 · menus"]
    editor --> menus
    library --> chrome["창 장식 · chrome"]
    editor --> chrome
    chrome --> visuals["시각 기반 · theme / icons / style"]
    editor --> visuals
    library --> state["공유 상태 · AppState"]
    cards -->|조회| state
    editor -->|편집·닫기 intent| state
    menus -->|삭제 intent| state
    library -->|시작| controller["저장·닫기 조율 · controller"]
    editor -->|변경·flush| controller
    controller -->|완료 반영| state
    controller -->|갱신| cards
    controller -->|생성 완료| editor
```

- 뷰와 controller는 GTK 스레드의 AppState를 공유한다. 뷰가 본문·색상·작업 intent를 직접 변경한다.
- 실패 표시는 `AppState::save_failed`, 편집 상태는 `EditorWindow::show_status`에서 공통 처리한다.
- 책임 분리 근거: ADR-0007. 저장 쪽 확장 뷰는 C3 persistence로 찾는다.
