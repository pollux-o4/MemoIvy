# ADR-0014 · dev 작업 이력과 main 출시 이력

- 맥락: 1인 개발. 사용자는 출시 이력만 보면 되고, 작업 중 커밋은 개발 확인용이다. 2026-11-24 release-please를 시험했으나 squash 후 릴리스 PR을 한 번 더 병합하는 이중 출시가 됐다.
- 결정: 브랜치는 `dev`(작업 이력)와 `main`(출시 이력·기본 브랜치) 둘만 상시 유지한다. `dev→main` PR의 squash 병합 하나가 곧 출시다.
- 출시 커밋: 제목 `release: MemoIvy <버전>`, 본문은 사용자에게 보이는 변경을 명사형 종결로 한 줄씩 적는다. 본문이 GitHub Release 노트가 된다. PR·병합 제목과 본문은 사용자 승인 후 사용한다.
- 버전: `package.json`이 SSoT다. `dev`에서 버전을 올린 뒤 병합하고, `scripts/release-plan.mjs`가 이미 출시된 버전을 CI(main 대상 PR)와 출시 워크플로에서 거부한다. hotfix도 패치 버전을 올려 같은 길로 출시하며 별도 브랜치를 두지 않는다.
- 출시 워크플로: `main` 푸시 → 초안 Release → CI 재통과 커밋에서 Windows·Snap 빌드·`scripts/verify-package.mjs` 검사·첨부 → 모두 성공 시 공개(태그 생성). 실패하면 공개하지 않는다.
- 동기화: 출시 뒤 `main`을 `dev`에 병합한다. `main`이 다시 만들어져 계보가 다르면 `-s ours --allow-unrelated-histories`로 코드 변경 없이 연결한다.
- 작업 브랜치: 기능·실험은 단기 브랜치·워크트리에서 해 `dev`에 병합한다. 병합되면 원격 브랜치는 자동 삭제하고 로컬 워크트리도 지운다. 병합 판정은 `git branch --merged`가 아니라 트리 비교·`git cherry`로 한다(squash·cherry-pick 반영분). 체크포인트는 브랜치 대신 태그로 남긴다.
- 대안: release-please(main에서 릴리스 PR 추가 병합 필요, squash 본문을 다시 Conventional Commits로 써야 함), `main` 단일 trunk(사용자에게 작업 커밋 노출), `release/*` 브랜치(다중 버전 지원용이라 과함).
- 실행·검증 절차: OPS-005. 브랜치·CI 개요: OPS-003.
