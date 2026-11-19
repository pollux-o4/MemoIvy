# RULE-ADR · 관심사별 결정 기록

- 배치: `decisions/<관심사>/ADR-0001--<키워드>--current.md`
  - 결정 하나당 맥락·대안·선택·이유·결과를 기록한다.
  - README는 관심사·탐색 키, 현재 결정은 `--current.md` 검색으로 찾는다.
- 파일명 상태
  - current: 현재 적용. 같은 관심사에 여러 current가 공존할 수 있다.
  - stale: 적용 종료. review: 유효성 재검토.
  - proposed: 제안. rejected: 기각.
- 대체
  - 의미 변경은 새 ID로 기록하고 새 ADR에 `대체: 이전 ID`를 남긴다.
  - 새 ADR 작성과 이전 파일의 stale 변경을 같은 변경 단위로 처리한다.
  - 이력은 기존 관심사 폴더에 보존한다.
- 검색 예: `rg --files decisions | rg '/gtk/.*--current\.md$'`
