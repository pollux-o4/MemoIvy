# C3 · 화면 컴포넌트

- 앱 조립은 `App`이 담당한다. 창별 화면을 선택하고 `useMemoivy`의 스냅샷과 명령을 기능에 연결한다.
- 기능은 목록·편집기·캔버스·첨부·설정·도움말로 나눈다. 기능이 공유하는 컨트롤과 Markdown 편집·미리보기는 `shared`에 둔다.
- Electron preload는 `DesktopBridge`의 데스크톱 어댑터다. Storybook과 브라우저 미리보기는 같은 화면에 mock 어댑터를 연결한다.
- 공통 SVG는 `frontend/shared/assets/icons`에 둔다. Storybook과 Electron 검증의 합성 데이터는 `fixtures`에서 관리한다.
