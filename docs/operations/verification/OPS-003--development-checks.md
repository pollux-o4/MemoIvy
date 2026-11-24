# OPS-003 · 개발 검증

- 변경한 영역은 `mise tasks`와 OPS-004에서 대응 검사를 찾는다. 엔진은 `mise run test:engine`, 데스크톱 통신은 `mise run test:desktop`, 화면 규칙은 `mise run test:web`을 사용한다.
- 타입·정적 검사는 `npm run typecheck`와 `mise run check:web`을 사용한다. 웹과 Storybook 배포 번들은 각각 `npm run build:web`, `npm run build:storybook`으로 확인한다.
- 화면 상호작용과 접근성은 `mise run test:stories`, 실제 Electron 창과 저장 복원은 `mise run test:e2e`로 확인한다. Electron 검사는 합성 데이터와 가상 X 화면을 사용한다.
- 캔버스 프레임·메모리 변화 판단은 OPS-004의 PERF 검사를 사용한다. 실제 GPU·Wayland 결과와 헤드리스 결과는 구별해 기록한다.
- 배포 전 이력 검사는 `mise run check:secrets`를 사용한다. 커밋·푸시 훅 설치는 `mise run hooks:install`이다.
- CI(`.github/workflows/ci.yml`)는 `dev` 푸시·PR마다 위 mise 작업을 그대로 실행한다: 정적 점검·단위 테스트, Linux·Windows Electron e2e, 비밀정보 검사. 검사 추가·변경은 mise 작업에 하고 워크플로는 호출만 한다. 도구 준비는 `.github/actions/setup` 하나를 공유한다. e2e 실패 시 화면 캡처를 산출물로 올린다.
- 의존성 갱신은 Dependabot이 주 1회 묶음 PR로 올리고, CI 통과 후 병합한다. 릴리스 흐름은 OPS-005.
