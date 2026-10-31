# 아키텍처 탐색

C1 시스템 → C2 실행·저장 단위 → C3 책임 모듈 → C4 코드 탐색 순서로 내려간다.

| 수준 | 관심사·탐색 키 |
|---|---|
| C1 Context | overview / 시스템 맥락 / 사용자·Ubuntu 세션 |
| C2 Container | overview / 실행·저장 단위 / 실행기·Rust GTK 앱·JSON |
| C3 Component | components / UI 조율·기능·표현 / 저장 정책·도메인·파일 I/O |
| C4 Code | code / UI·저장·실행 통합 / 대표 심볼과 구현 경로 |

C4는 수동 클래스 다이어그램 대신 코드 탐색을 제공한다. 구조는 코드, 선택 이유는 ADR, 동작 계약은 명세를 원본으로 유지한다. Python은 외부 검증·명령 호출 도구이며 앱의 도메인 규칙을 소유하지 않는다.
