# C3 · UI 컴포넌트

```mermaid
flowchart TB
    run["앱 조립 · run"] --> shell["shell · 명령·창 수명·저장 이벤트 조율"]
    subgraph features["기능별 화면·세션"]
        library["library · 검색·목록"]
        editor["editor · 원문 편집·서식·링크 카드"]
        canvas["canvas · 배치·연결·선택·모션"]
    end
    shell --> library
    shell --> editor
    shell --> canvas
    library -->|명령| shell
    editor -->|명령| shell
    canvas -->|명령| shell
    shell --> notebook["persistence 공개 계약 · Notebook"]
    canvas --> port["canvas::port · 읽기 모델·저장 연결"]
    port --> notebook
    library --> shared["shared · 창 장식·메뉴·포커스·단축키"]
    editor --> shared
    canvas --> shared
    library --> presentation["presentation · Markdown 해석·테마·아이콘·CSS"]
    editor --> presentation
    canvas --> presentation
    shared --> presentation
```

- `AppState`는 `Notebook`과 `Views`를 소유한다. `Closing`과 삭제 대기열은 저장 결과에 맞춰 창 종료·다음 삭제를 진행한다.
- 편집 원문은 `TextBuffer`에 두고 서식·목록 표식·링크 카드를 파생시킨다. 캔버스는 `port`의 읽기 모델로 카드와 패널을 구성한다.
- GTK 독립 Markdown 해석·요약은 편집기와 캔버스가 공유한다. 표식 기호는 같은 상수, 체크 색·테두리는 공통 CSS 선택자를 쓴다.

기능 간 조율·저장 경계: ADR-0010. 캔버스 상호작용 설계: ADR-0009.

## 캔버스 확대

```mermaid
flowchart TB
    facade["canvas · 기존 화면 진입점·세션"] --> controller["controller · 조율"]
    controller --> window["window · 창 조립·수명"]
    controller --> commands["commands · 변경·이력"]
    controller --> camera["camera · 뷰포트"]
    controller --> view["view · 선택·화면 동기화"]
    commands --> port["port · 저장 어댑터"]
    port --> notebook["Notebook · 원본"]
    camera --> model["model · GTK 없는 계산"]
    view --> scene["scene · 보드·그리기·카드"]
```

`controller`는 기존 `CanvasView`의 조율 책임을 나눠 구현한다. 저장 원본은 Notebook에만 두고 세션의 위치·카드는 화면용 파생 상태로 유지한다. 모델 내부는 geometry(카메라·배치·정렬), selection, history, spring으로 탐색한다. C4 UI 탐색에서 코드 진입점을 찾는다.
