# C3 · 저장 컴포넌트와 스레드 경계

```mermaid
flowchart LR
    shell["프런트 shell"] -->|변경·flush·tick| notebook["Notebook · 원본·디바운스·트랜잭션"]
    notebook -->|읽기 전용 Note·이벤트·상태| shell
    notebook --> graph["domain · 메모·연결·좌표 불변식"]
    notebook --> queue["storage::SaveQueue · revision / durable"]
    queue -->|mpsc snapshot| writer["단일 writer thread"]
    writer --> store["storage::store · 원자 저장"]
    store --> file[("notes.json")]
    file -->|검증된 로드| notebook
    writer -->|결과| queue
```

- Notebook은 GTK 스레드에서 사용하되 GTK 타입을 받지 않는다. writer·채널·revision은 비공개다.
- 생성·삭제는 파일 쓰기 성공 후 원본에 반영하고, 진행 중 다른 편집을 병합해 최신 스냅샷을 저장한다.
- 본문·색상은 300ms 디바운스·1초 상한을 적용한다. flush는 즉시 저장을 요청하며 실패 후 재시도도 맡는다.
- 프런트는 `needs_tick`이 참일 때만 결과를 폴링하고, 오류 문구와 창 닫기는 직접 처리한다.
- 저장 형식·단일 writer·내구성 계약: DATA-001, DATA-002, ADR-0004. 정상 닫기·실패 복구: SAVE-001, SAVE-002.

- 첨부 파일은 Notebook과 나란한 `Attachments`가 맡는다. 이미지 바이트는 GTK 스레드 밖에서 `AssetStore`로 원자 저장하고, 사용처 색인은 저장·삭제 이벤트 뒤 shell이 맞춘다. writer 큐를 거치지 않으며 메모 저장을 막지 않는다(DATA-003, ADR-0011).
- 내부 탐색은 `domain`(note·canvas·links), `notebook`(명령·저장 정책), `storage`(paths·save_queue·store), `attachments`(files·usage)로 내려간다. `valid_position`이 편집과 파일 로드의 공통 좌표 규칙이다. C4 저장 탐색에서 구현 심볼을 찾는다.

- Notebook 내부에서 `CanvasEdit`은 캔버스 변경·불변식 적용을, `Operation::apply`는 저장 후보와 성공 후 원본의 동일한 생성·삭제 적용을 맡는다. writer 완료 전 생성·삭제를 게시하지 않는 순서는 유지한다.
