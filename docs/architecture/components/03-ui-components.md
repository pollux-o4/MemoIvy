# C3 · UI 컴포넌트

```mermaid
flowchart TB
    lifecycle["앱 조립 · run"] --> views
    subgraph views["창"]
        library["목록 · library"] --> editor["원본 메모 · editor"]
        library --> canvas["시각 흐름 · CanvasView"]
        editor --> canvas
    end
    subgraph controls["내용·조작"]
        cards["목록·삭제 · cards / menus"]
        editing["실시간 서식 · live_editor / formatting"]
        spatial["공간·입력 · board / interaction"]
        feedback["선택·전환 · inspector / motion"]
    end
    library --> cards
    editor --> editing
    canvas --> spatial
    canvas --> feedback
    views --> state["단일 원본 · AppState / Store.notes"]
    views --> controller["변경·저장·닫기 · controller"]
    controller -->|결과 반영| state
    controller -->|뷰 갱신| views
    spatial --> graph["그래프 정책 · persistence::canvas"]
    views --> visuals["공유 시각·입력 기반 · chrome / theme / icons / style / focus / shortcuts"]
```

- 창은 동일 메모 UUID를 공유한다. 편집기는 본문·색상, 캔버스는 배치·방향 연결·완료를 변경하며 원문을 복제하지 않는다.
- CanvasView는 창과 세션 상태를 조립한다. board는 뷰포트 렌더링, interaction은 카드 입력, inspector는 선택 작업, motion은 일시적인 확대·신호 전환을 맡는다.
- 확대는 기존 위젯의 변환만 갱신한다. GTK 프레임 콜백은 활성 전환 중에만 유지하며 모션 줄이기 설정에서는 즉시 전환·정적 도착 표시를 사용한다.
- 본문 서식과 원문 변환은 presentation::markdown이 파싱하고 live_editor·formatting이 적용한다. focus·shortcuts는 창 간 입력 소유권과 공통 키보드 정책을 공유한다.
- 뷰와 controller는 GTK 스레드의 AppState를 공유한다. 저장 실패는 AppState::save_failed, 편집 상태는 EditorWindow::show_status, 캔버스 상태는 CanvasView::update_status가 표시한다.

책임 분리: ADR-0007, ADR-0008. 동작·모션: CANVAS-001, CANVAS-002. 저장 경계는 C3 persistence로 찾는다.
