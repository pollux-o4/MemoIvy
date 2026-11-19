# OPS-005 · Windows와 Snap 데스크톱 배포

## 산출물

- `package.json`의 릴리스 버전을 사용한다. Windows와 Snap은 같은 커밋을 빌드한다. `Desktop release candidates`는 설치본, `SHA256SUMS`, `SOURCE_COMMIT`을 보관한다. 업로드와 채널 변경은 이 빌드 작업에서 실행하지 않는다.
- Windows x64는 `npm run release:windows`로 사용자별 NSIS 설치본을 만든다. Linux에서는 다른 Wine 접두사와 가상 화면에서 설치만 먼저 확인한다. 실제 Windows에서 표준 사용자·한글 설치 경로·앱 메뉴·작성·첨부·재시작·업데이트를 확인한다.
- Store용 MSIX는 Windows SDK와 `scripts/package-msix.ps1`을 사용한다. Partner Center에서 할당한 이름, 게시자, 표시 이름을 각각 `MSIX_IDENTITY`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_NAME`에 설정한다. 임시 개발 식별자로 만든 패키지는 제출 후보에서 제외한다.
- Snap은 Ubuntu 24.04에서 Snapcraft·LXD를 준비한 뒤 `npm run release:snap`을 사용한다. `unsquashfs -ll`로 엔진을 확인하고 `unsquashfs -cat 후보.snap meta/snap.yaml`로 이름·격리·연결 권한을 확인한다.

## 후보 검증

- Windows 직접 설치본의 Authenticode 서명과 체크섬을 확인한 뒤 버전이 고정된 주소에 올린다. Store용 MSIX는 전용 Windows 검수 환경에서 패키지를 설치하고 앱 인증 검사와 제한 배포를 거친다. 직접 설치판과 Store판의 저장 경로가 달라지면 데이터 복구를 확인한 뒤 배포한다.
- 격리 Linux에서 빈 사용자로 Snap 새 설치를 확인한다. 두 번째 사용자에 이전 GTK 리비전을 설치하고 합성 메모·첨부를 작성한 뒤 앱을 종료한다. 이전 원본과 첨부의 체크섬을 기록한다.
- 후보 채널에서 갱신한 뒤 새 앱을 실행한다. 목록, 메모 본문, 이미지, 편집·재시작 복원을 확인하고, 원본과 백업의 체크섬을 비교한다. 이때 기록한 후보 리비전을 정식 배포에 사용한다.
- 앱 종료 후 공유 SQLite 폴더를 복사한다. 이전 리비전으로 되돌려 JSON과 첨부를 확인하고, 검증한 후보를 다시 적용해 새로 작성했던 SQLite 메모를 확인한다(OPS-002).

## 게시와 복구

- 서명과 Store·Snap 게시 자격 증명은 권한을 제한한 작업 환경에서 사용한다. 테스트 패키지와 후보 체크섬을 검토한 뒤 선택한 산출물만 게시한다.
- Snap은 `snapcraft upload 후보.snap --release=latest/candidate` 후 위 검증을 진행한다. 통과한 리비전만 `snapcraft release memoivy 리비전 latest/stable`로 승격한다.
- 장애 시 게시 담당자가 이전 리비전을 채널에 재지정하고, 장치에서 `snap revert memoivy`와 OPS-002 복구 절차를 확인한다. 새 SQLite 파일은 별도로 보존한다.
