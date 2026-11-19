# C4 Code · 웹 캔버스 탐색

React 캔버스(`frontend/features/canvas`)는 조립, 조작, 순수 모델, 화면 부품의 네 묶음으로 구성한다. 상위 묶음만 하위 묶음을 참조하며(조립 → 조작 → 모델), 화면 부품은 렌더링과 화면 내부 조작을 맡는다.

## 조립

| 관심사 | 탐색 키 |
|---|---|
| 화면 조립 | `Canvas`: 범위·선택·카메라·명령·포인터 훅을 연결하고 결과를 화면 부품에 전달한다. |
| 카드 배치 계산 | state의 `useCanvasLayout`: 저장된 위치에 끌기 중인 임시 위치를 반영해 카드 위치와 영역을 한곳에서 계산한다. |

## 조작 · `interaction/`

| 관심사 | 탐색 키 |
|---|---|
| 카메라 | camera의 `useCanvasCamera`(카메라 기능의 단일 진입점, 단계별 위치·확대율 복원과 복귀 카드 표시), `useCanvasMotion`·`useCanvasWheel`·`useRevealSelection`·`useBoardPoint` |
| 입력 | input의 `useCanvasPointer`(조작 모드별 포인터 처리), `useConnectGesture`(연결 끌기·대상 선택), `useCanvasKeyboard`·`useCardMenu` |
| 명령·이력 | commands의 `useCanvasCommands`·`useCanvasHistory`·`useCanvasClipboard`·`useSelectionActions`(본문 저장·체크 전환·선택 명령) |
| 화면 상태 | state의 `useCanvasSelection`·`useCanvasScope`·`useBranchFocus`·`useFanQuiet`·`useStableHandlers` |

## 순수 모델 · `model/`

| 관심사 | 탐색 키 |
|---|---|
| 카드 기하·저장 좌표 | `align`(진입점) → geometry의 `CARD_WIDTH`·`CANVAS_LIMIT`·`CANVAS_MIN`(출처 `desktop/canvas_bounds.json`, IPC 검증·엔진과 공유)·`clampCanvasDelta`, savedPositions의 `savedPoint`·`settleDraft`, alignment의 `alignPositions` |
| 카메라·물리 | `motion`(진입점) → geometry의 `fitting`·`zoomAt`, physics의 `Spring`·`Coast`, cameraMotion, signals의 `pulseSample` |
| 끌기·선택 카드 표시 | drag의 `dragDelta`·`releaseVelocity`, reveal의 `visibleArea`·`revealCamera` |
| 연결선 | links의 `graph`·`anchors`·`paths`(`PATH_STRATEGIES`)·`mindmap_layout`·`outline_tree`(탐색)·`outline_edit`(변경) |

## 화면 부품 · `parts/`

| 관심사 | 탐색 키 |
|---|---|
| 캔버스 그리기 | `CanvasWorld`(격자·선·카드·미리보기·정렬 안내선), `CanvasCard`·`CanvasLinks`·`ConnectPreview`. 카드 하단은 `CardSummary`와 `card_footer`에 모으고 공통 `TaskProgress`를 연결한다. 본문 미리보기는 카드·인스펙터·목록이 공유하는 `markdown/preview`에서 파싱·표시·체크 저장을 맡는다. 연결점은 `useCardHeights`가 변경된 카드만 관찰해 반영한다. 고속 휠 확대는 포인터 기준으로 입력을 순차 적용하고, 화면 갱신은 프레임당 한 번으로 합친다. 카드는 확대 중 재조립하지 않는다. 본문 높이는 `useCollapsible`이 클리핑 영역 대신 행을 관찰해 계산한다. |
| 떠 있는 패널 | inspector의 `CanvasInspector`·`InspectorOutline`·`useOutlineDrag`·`LinkPicker`(버튼과 Ctrl+L의 공통 메뉴), `CanvasDock`·`CardMenu`·`CanvasNavigator` |

동작 계약은 CANVAS-001~005에서, 실행·검증 절차는 OPS-004에서 찾는다.
