# OPS-005 · Windows와 Snap 데스크톱 배포

## 산출물

- 브랜치·버전·출시 커밋 규칙: `decisions/release`의 current ADR.
- 출시 절차: `dev`에서 `npm version <버전> --no-git-tag-version` 커밋 → `dev→main` PR squash 병합 → `Release` 워크플로 성공·Release 공개 확인 → `main`을 `dev`에 병합 → Windows VM 설치 확인 → 스토어 승격.
- `Release` 워크플로는 `scripts/release-plan.mjs`로 태그를 정하고 초안 Release를 만든 뒤 `Desktop release candidates`를 호출한다. `scripts/release-ci-gate.mjs`가 병합 PR 머리 커밋의 `dev` 푸시 실행에서 같은 코드가 CI 전체(Linux·Windows e2e 포함)를 통과했는지 확인하고, 확인할 수 없으면 CI를 다시 실행한다. 후보 빌드는 이 관문을 통과한 커밋에서 Windows와 Snap을 빌드한다. `scripts/verify-package.mjs`가 대상별로 엔진 동봉·Windows 엔진의 VC 런타임 비의존(CRT 정적 링크, `.cargo/config.toml`)·Snap 단일 산출물을 확인한다. 대상별 `SHA256SUMS-*`·`SOURCE_COMMIT-*`와 빌드 출처 증명을 남기고 산출물을 해당 Release에 붙인다. 모두 성공해야 Release가 공개되고 태그가 생긴다. 태그 없이 수동 실행하면 산출물만 보관한다. 스토어 업로드와 채널 변경은 이 작업에서 실행하지 않는다.
- Windows 검증 단계: 매 푸시의 CI Windows e2e가 기능 회귀를 막고, 공용 Windows VM은 릴리스 후보의 설치·Store·첫 실행 확인에만 사용한다.
- Windows x64는 `npm run release:windows`로 사용자별 NSIS 설치본을 만든다. Linux에서는 다른 Wine 접두사와 가상 화면에서 설치만 먼저 확인한다. 실제 Windows에서 표준 사용자·한글 설치 경로·앱 메뉴·작성·첨부·재시작·업데이트를 확인한다.
- Store용 MSIX는 Windows SDK와 `scripts/package-msix.ps1`을 사용한다. Partner Center에서 할당한 이름, 게시자, 표시 이름을 각각 `MSIX_IDENTITY`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_NAME`에 설정한다. 임시 개발 식별자로 만든 패키지는 제출 후보에서 제외한다.
- Snap은 Ubuntu 24.04에서 Snapcraft·LXD를 준비한 뒤 `npm run release:snap`을 사용한다. 엔진은 후보 빌드가 확인하고, `unsquashfs -cat 후보.snap meta/snap.yaml`로 이름·격리·연결 권한을 확인한다.

## 후보 검증

- Windows 직접 설치본의 Authenticode 서명과 체크섬을 확인한 뒤 버전이 고정된 주소에 올린다. Store용 MSIX는 전용 Windows 검수 환경에서 패키지를 설치하고 앱 인증 검사와 제한 배포를 거친다. 직접 설치판과 Store판의 저장 경로가 달라지면 데이터 복구를 확인한 뒤 배포한다.
- 격리 Linux에서 빈 사용자로 Snap 새 설치를 확인한다. 두 번째 사용자에 이전 GTK 리비전을 설치하고 합성 메모·첨부를 작성한 뒤 앱을 종료한다. 이전 원본과 첨부의 체크섬을 기록한다.
- 후보 채널에서 갱신한 뒤 새 앱을 실행한다. 목록, 메모 본문, 이미지, 편집·재시작 복원을 확인하고, 원본과 백업의 체크섬을 비교한다. 이때 기록한 후보 리비전을 정식 배포에 사용한다.
- 앱 종료 후 공유 SQLite 폴더를 복사한다. 이전 리비전으로 되돌려 JSON과 첨부를 확인하고, 검증한 후보를 다시 적용해 새로 작성했던 SQLite 메모를 확인한다(OPS-002).

## 게시와 복구

- 서명과 Store·Snap 게시 자격 증명은 권한을 제한한 작업 환경에서 사용한다. 테스트 패키지와 후보 체크섬을 검토한 뒤 선택한 산출물만 게시한다.
- GitHub Actions에 `SNAPCRAFT_STORE_CREDENTIALS`를 등록하고 저장소 변수 `SNAP_STORE_CANDIDATE_ENABLED`를 `true`로 설정하면 GitHub Release 공개 후 Snap을 `latest/candidate`에 올린다. 토큰은 `memoivy`와 `latest/candidate`에 한정한다.
- 재실행하려면 `Snap Store candidate` 워크플로(어느 브랜치에서든)에 최신 공개 태그를 지정한다. 두 스토어 워크플로는 `.github/actions/release-asset`으로 최신 공개 Release의 설치 파일을 받아 SHA-256을 확인한다. 위 검증을 통과한 리비전은 게시 담당자가 `snapcraft release memoivy 리비전 latest/stable`로 승격한다.
- Microsoft Store 첫 공개는 Partner Center에서 진행한다(완료). 이후에는 GitHub Release 공개 뒤 `Microsoft Store submission` 워크플로가 체크섬을 확인한 MSIX를 `msstore publish`로 새 제출에 올리고 커밋한다. 시크릿 `MSSTORE_TENANT_ID`·`MSSTORE_CLIENT_ID`·`MSSTORE_CLIENT_SECRET`, 변수 `MSSTORE_SELLER_ID`·`MSSTORE_PRODUCT_ID`가 필요하고, `MSSTORE_PRODUCT_ID`가 비면 건너뛴다. Entra 앱은 Partner Center 사용자 관리에 관리자 권한으로 연결돼 있어야 한다. MSIX 신원은 변수 `MSIX_IDENTITY`·`MSIX_PUBLISHER`·`MSIX_PUBLISHER_NAME`이 Partner Center 값과 같아야 한다.
- 재제출은 `Microsoft Store submission` 워크플로(어느 브랜치에서든)에 최신 공개 태그를 지정한다.
- 장애 시 게시 담당자가 이전 리비전을 채널에 재지정하고, 장치에서 `snap revert memoivy`와 OPS-002 복구 절차를 확인한다. 새 SQLite 파일은 별도로 보존한다.

## 공용 배포 검증 환경

- 공용 공간의 `ReleaseQA` README에서 베이스 이미지·변경분 디스크·프로젝트별 검증 자료를 찾는다. 로컬에서는 저장소와 같은 상위 폴더에 둔다.
- Windows VM에서 Store 설치·첫 실행을, 격리 Ubuntu에서 Snap 설치·갱신·롤백을 확인한다. 검증 기록과 변경분 디스크는 `projects/MemoIvy`에 둔다.
- 성공 확인 후 커밋·패키지 체크섬·채널 또는 리비전·검증 결과를 남기고 임시 디스크와 빌드 중간물을 정리한다. 실패 재현에 필요한 변경분은 원인 확인까지 보관한다.
