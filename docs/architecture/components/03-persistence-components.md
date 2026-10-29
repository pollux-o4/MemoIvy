# C3 · 저장 컴포넌트와 스레드 경계

```mermaid
flowchart LR
    subgraph main["GTK main thread"]
        load["Store::load"]
        state["메모·캔버스 원본 · Store.notes"]
        controller["controller"]
        queue["SaveQueue · revision / durable"]
    end
    subgraph worker["단일 worker thread"]
        writer["순차 작업 루프"]
        save["store::save"]
    end
    file[("notes.json")]
    load -->|읽기| file
    load -->|검증한 메모| state
    graph["그래프 검증·연결·삭제 정리 · canvas"] --> state
    controller -->|snapshot 구성| state
    controller -->|dispatch·poll| queue
    queue -->|mpsc snapshot| writer
    writer --> save
    save -->|동기화·원자 교체| file
    writer -->|mpsc 결과| queue
    controller -->|결과 반영| state
```

- Note.canvas는 위치·다음 메모 UUID·완료 상태를 담는다. 그래프 검증은 로드·저장 경계에서 수행하고 메모 삭제는 참조 연결을 같은 transaction에서 제거한다.
- 로드는 시작 시 GTK 스레드, 쓰기는 worker에서 수행한다.
- 일반 저장은 revision, 생성·삭제 결과는 transaction 표식으로 구분한다.
- 스레드·저장 조율 근거: ADR-0004. 내구성 계약: DATA-002. 정상 닫기·실패 복구: SAVE-001, SAVE-002.
