# MemoIvy 소개 영상

Snap Store용 40초 영상(1920×1080)과 스크린샷 `../screenshots/01~04.png`의 제작 원본.

- 편집 기준: `web/film.js`(비트·자막·카메라·동선). 이야기·샷 구성은 `editorial/TREATMENT.md`.
- 앱 화면 문구·색은 `frontend/shared/tokens.css`·Storybook을 따른다. 앱 UI가 바뀌면 `web/ui.js`도 맞춘다.
- 출력: `./build.sh` → `exports/`. mp4는 Git에 넣지 않는다. Lemo-Opuscar 라이브러리가 필요하며 경로가 다르면 `LIB=<경로> ./build.sh`.
