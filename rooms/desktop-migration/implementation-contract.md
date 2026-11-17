# MIG-001 구현 계약 · v1

소유 경계: engine/ 전용 Rust, frontend/ 전용 React, desktop/ 전용 Electron. root 설정·의존성·최종 문서·기존 GTK 정리는 상위 담당. 커밋 금지. 다른 담당 파일 변경 금지. 공개 변경은 이번 작업에서 승인됨.

통신 원본: frontend/shared/engine_contract.ts (상위 담당 소유, 하위 수정 금지). 한 줄 JSON stdio. 요청 {version:1,id,op,params}. 응답 {version:1,id,result} 또는 error {code,message,snapshot?}. 이벤트 {version:1,event:'snapshot',snapshot} 또는 'error'. stdout는 프로토콜만, 로그 stderr. 모든 명령 결과 result.snapshot 포함 권장. snapshot {notes,assets,revision,savedRevision}. 성공 write 응답은 SQLite commit 이후, revision=savedRevision. 변경 후 snapshot 이벤트. 충돌은 code CONFLICT와 최신 snapshot, 초안 보존.

Note는 기존 JSON 형식 id,text,color,modified,canvas를 유지. canvas {position:null|[x,y],next:[],done,project,parent,pinned}. revision은 전역 증가. 변경 명령 params.baseRevision 필수. read 명령은 생략. SQLite 원본은 앱 데이터 폴더 memoivy.sqlite3. engine 실행 --data-dir ABS_PATH, --legacy ABS_NOTES_JSON (선택). 파일 잠금으로 엔진 동시 실행 차단. JSON 원본 무수정, 백업·검증 후 DB 트랜잭션으로 1회 import. 기존 GTK를 유지하는 동안 같은 경로 notes.json의 OS 파일 lock도 엔진 수명 동안 획득; 기존 GTK lock은 상위가 추가.

작업:
- snapshot: {} -> snapshot.
- create: {baseRevision,text?:'',color?:'yellow',parent?:null,project?:false,position?:null} -> note,snapshot.
- update: {baseRevision,id,text?,color?,canvas?:부분 CanvasNode} -> note,snapshot. canvas update는 domain 불변식 검증.
- delete: {baseRevision,id} -> snapshot. 자식 승격·incoming edge 제거.
- canvas: {baseRevision,changes:[{id,canvas:부분 CanvasNode}]} -> snapshot. 한 transaction, undo/redo는 frontend snapshot 재적용.
- flush: {} -> snapshot (모든 처리된 변경 durable 확인).
- importLegacy: {path} -> snapshot. 빈 DB에만 import, attachments도 기존 폴더에서 보존/복사. DB가 비어 있지 않으면 안전하게 거부.
- importAsset: {baseRevision,name,data:base64,mime?} -> asset,snapshot. size<=20MiB, SHA256 ID, 허용 이미지 또는 일반 파일(실행하지 않음).
- readAsset: {id} -> {data:base64,mime}.
- deleteAsset: {baseRevision,id} -> snapshot. 사용 중이면 거부.

renderer: 브라우저 Storybook/mock fallback. 실제 앱 window.memoivy DesktopBridge. snapshot 먼저 subscribe 붙인 뒤 로드하고 revision 역전 무시. autosave 300ms·1초 상한, IME 조합 후 반영, 초안 보존. 충돌시 자동 덮어쓰기 금지. Ctrl+Shift+M 원문, Ctrl+Enter 체크, 독립 편집창·캔버스·검색·프로젝트·첨부 기능 이관. 외부 링크는 bridge.openExternal, raw HTML 실행 금지. 기존 UI-001 disclosure 재사용 및 실제 본문 UI 확인.

desktop: single instance, main이 engine spawn·IPC 중개, 모든 창 동일 engine. main이 스냅샷 캐시와 이벤트 broadcast. preload contextBridge 작업별 제한 API, sender·params validate. onFlush로 창의 초안 저장 완료를 기다린 후 close. failed/time out시 창 유지하고 오류 이벤트. 기본 데이터 dir: Linux XDG 또는 ~/.local/share/memoivy, macOS appData/MemoIvy, Windows appData/MemoIvy. MEMOIVY_DATA_DIR 절대경로 override 합성 검증용. 자동 기존 Linux/Snap JSON 탐색은 읽기만 하고 원본 보호. development renderer http://127.0.0.1:5173, production frontend/dist/index.html. dev engine engine/target/debug/memoivy-engine, packaged resources/engine/memoivy-engine(.exe). desktop/main.cjs 및 preload.cjs, builder 설정 desktop/electron-builder.yml. main 종료 시 engine flush 후 stdin 종료. importLegacy는 file picker로 path를 승인 후 전달.

최소 검증: 엔진 migration·rollback·conflict/reopen 테스트, 프런트 타입/Storybook 빌드, Electron 합성 데이터 편집·재시작 복원. root scripts/dependencies는 필요한 목록을 상위에게 메시지로 요청. 전체 OS 검사 체계 추가 금지. 데이터 보호/기능 완료에 필요한 작업은 shortcut으로 미루지 말 것.

추가 계약: DesktopBridge.openWindow(kind:'library'|'canvas'|'files',id?) 사용. renderer query는 ?window=kind&id=uuid, 편집기는 window=note. snapshot.preferences에는 expandedProjects/expandedNotes UUID 배열, navigation {uuid:{count,lastVisited}}, autoPrune boolean. preference 명령 {baseRevision,key,value}. Asset의 added/lastNote/unusedSince는 파일 화면 이력을 제공. 기본 autoPrune=false.

충돌 보완: baseRevision은 항상 사용자가 작업을 시작할 때 관찰한 전역 snapshot.revision이다. 엔진은 대상 noteRevisions[id]가 baseRevision 이후 바뀌었을 때만 충돌을 반환한다. 다른 메모·선호 변경은 본문 저장을 막지 않는다. 편집 초안은 처음 더러워진 시점의 baseRevision을 유지하며 서버의 새 snapshot만으로 기준을 올리지 않는다. 드래그도 pointerdown 시점을 사용한다. 요청 함수는 명시한 baseRevision을 기본값으로 덮지 않는다.
