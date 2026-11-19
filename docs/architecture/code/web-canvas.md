# C4 Code · 웹 캔버스 탐색

React 캔버스(`frontend/features/canvas`)는 조립·조작·순수 모델·화면 부품의 네 묶음으로 나눈다. 의존 방향은 조립 → 조작 → 모델이며, 화면 부품은 받은 값만 그린다.

## 조립

| 관심사 | 탐색 키 |
|---|---|
| 화면 조립 | `Canvas`: 범위·선택·카메라·명령·포인터 훅을 묶어 부품에 전달한다. |
| 파생 배치 | state의 `useCanvasLayout`: 저장 위치 위에 끌기 초안을 덮은 위치·카드 사각형의 단일 출처 |

## 조작 · `interaction/`

| 관심사 | 탐색 키 |
|---|---|
| 카메라 | camera의 `useCanvasCamera`(파사드), `useCanvasMotion`·`useCanvasWheel`·`useRevealSelection`·`useBoardPoint` |
| 입력 | input의 `useCanvasPointer`(모드별 상태 기계), `useConnectGesture`(끌기·고르기), `useCanvasKeyboard`·`useCardMenu` |
| 명령·이력 | commands의 `useCanvasCommands`·`useCanvasHistory`·`useCanvasClipboard`·`useSelectionActions` |
| 화면 상태 | state의 `useCanvasSelection`·`useCanvasScope`·`useBranchFocus`·`useFanQuiet`·`useStableHandlers` |

## 순수 모델 · `model/`

| 관심사 | 탐색 키 |
|---|---|
| 카드 기하·저장 좌표 | `align`(진입점) → geometry의 `CARD_WIDTH`·`CANVAS_LIMIT`·`clampCanvasDelta`, savedPositions의 `savedPoint`, alignment의 `alignPositions` |
| 카메라·물리 | `motion`(진입점) → geometry의 `fitting`·`zoomAt`, physics의 `Spring`·`Coast`, cameraMotion, signals의 `pulseSample` |
| 끌기·드러내기 | drag의 `dragDelta`·`releaseVelocity`, reveal의 `visibleArea`·`revealCamera` |
| 연결선 | links의 `graph`·`anchors`·`paths`(`PATH_STRATEGIES`)·`mindmap_layout`·`outline_edit` |

## 화면 부품 · `parts/`

| 관심사 | 탐색 키 |
|---|---|
| 세계 그리기 | `CanvasWorld`(격자·선·카드·미리보기·안내선), `CanvasCard`·`CanvasLinks`·`ConnectPreview` |
| 떠 있는 패널 | inspector의 `CanvasInspector`, `CanvasDock`·`CardMenu`·`CanvasNavigator` |

동작 계약은 CANVAS-001~005, 실행·검증 절차는 OPS-004로 찾는다.
