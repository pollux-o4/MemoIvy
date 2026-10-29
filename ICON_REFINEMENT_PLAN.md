# 아이콘 일관성 개선

## 승인 범위

최신 사용자 피드백은 “모양이 제각각”, “신호등 안에 뭐가 들어 있어서 더 구리다”, “커스텀 깔끔한 트렌디한 SVG”. 이전 `DESIGN_PLAN.md`의 신호등 원·빨강/노랑/초록·문자 기호 지시는 이 문서로 대체한다. 화면 구조, 메모 카드, 저장 방식과 기능은 바꾸지 않는다. 아래 수치는 요청을 구현하는 디자인 판단이다.

## 한 가지 아이콘 문법

- SVG `viewBox="0 0 20 20"`, 기본 표시 20×20, `fill="none"`, stroke 1.75, round linecap/linejoin. 최대화·복원 사각형의 모서리 반경 1.5. 곡선/선은 대략 좌표 4~16의 동일한 시각 영역에 배치한다.
- 창 조작은 왼쪽의 현재 위치를 유지하되 컬러 원과 ×/−/+ 텍스트를 완전히 제거한다. 닫기: 대각선 두 선, 최소화: 가운데 수평선, 최대화: 둥근 사각형, 복원: 겹친 두 사각형. 실제 최대화 여부에 따라 아이콘·tooltip·접근 가능한 이름을 함께 전환한다.
- 팔레트: 작은 물감판 outline와 2~3개의 작은 점. 더 보기: 같은 viewBox 가운데 가로 점 3개(반경 1); 더 보기 점은 명확히 의도된 채움 도형으로 예외 허용. 추가: 동일 stroke의 plus SVG와 “새 메모” 텍스트. `+ 새 메모` 텍스트의 plus는 없앤다.
- 헤더와 카드의 더 보기 아이콘은 동일 파일을 쓴다. window button, palette, more, add에 시스템 icon name이나 글꼴 문자로 된 모양을 섞지 않는다. 색 선택의 체크는 선택 상태이므로 창 조작과 무관하지만 가까운 버튼 외형을 해치면 동일 stroke의 20×20 check SVG를 재사용한다.
- 검색·빈 상태 아이콘은 이번 문제의 핵심이 아니다. 변경한다면 간단한 search/document SVG를 같은 규격으로 추가하는 범위까지만 허용한다. SearchEntry의 네이티브 입력·지우기 동작을 다시 구현하지 않는다.

## 버튼과 색상

- 아이콘 버튼 hit area 36×36, 시각 아이콘 20×20, 버튼 모서리 8px. 아이콘 자체에 둥근 배경판을 그리지 않는다. 그룹 간격 0~4px. 기존 헤더의 위계와 가운데 제목을 유지한다.
- 기본 배경 transparent, border/background-image/box-shadow 없음. light foreground #39414B, dark #E2E6EC. 강조된 새 메모 버튼 안의 plus는 흰색. 메모 종이색 위에서도 같은 foreground를 사용한다.
- hover는 light에서 검정 5~7%, dark에서 흰색 7~9%의 얕은 표면. close hover만 절제된 붉은 반투명 배경으로 표시할 수 있다. 이때도 빨강 원이 되지 않는다. 클릭 시 살짝 더 진한 동일 계열. 색 변화는 최대 100ms이며 필수 애니메이션은 없다.
- keyboard focus는 기존 명확한 2px ring 유지. 창이 inactive인 상태는 disabled가 아니며 버튼 기능과 대비를 유지한다. 실제 민감도 비활성화와 혼동하는 일괄 opacity 처리는 하지 않는다.
- `.traffic`와 그 label/class별 중복 override는 제거하고 단일 `.window-action` 규칙으로 교체한다. 기존 `.icon-button`과 window-action이 같은 치수·모서리·hover 토큰을 공유하게 한다. Text label을 숨겨 남기는 방식은 금지한다.

## GTK4 구현 방식

- 직접 작성한 작은 SVG를 `src/icons/`에 저장하고 `include_str!` 또는 `include_bytes!`로 바이너리에 내장한다. 런타임 작업 디렉터리·사용자 icon theme·별도 설치 경로에 의존하지 않는다. 새 크레이트나 GResource 빌드 파이프라인은 이 작업에 필요하지 않다.
- 현재 설치된 gtk4 0.9.7은 `gtk::gdk_pixbuf`를 재노출한다. 로컬 crate 소스에서 `Pixbuf::from_read`, `from_stream_at_scale`과 `gdk::Texture::for_pixbuf` 존재를 확인했다. 가장 작은 방법은 SVG의 통제된 색 토큰을 foreground 색으로 치환해 메모리에서 Pixbuf로 읽고 Image/Paintable로 표시하는 것이다. 임의 본문이나 외부 데이터를 SVG에 삽입하지 않는다.
- 픽셀 이미지로 만든 뒤에는 CSS `color`가 자동 tint한다고 가정하지 않는다. light/dark, 새 메모 흰색 변형을 실제 SVG stroke에 반영한다. 테마 전환 시 이미 열린 창의 이미지도 갱신한다. close hover는 배경만 바꾸면 별도 아이콘 색 variant가 필요 없다.
- SVG는 scale factor에 맞는 크기로 decode하여 논리 20px로 표시한다. 2× 화면에서 20px bitmap을 확대하지 않는다. 렌더 시 크기나 intrinsic size 때문에 버튼이 커지지 않는지 확인한다. 테마/scale 변경 때 갱신하되 매 프레임 SVG를 다시 읽지 않는다. 소수 아이콘을 위한 범용 icon manager는 만들지 않는다.
- SVG loader가 없거나 parse가 실패하면 조용히 빈 버튼 또는 시스템 아이콘으로 돌아가지 않는다. 개발 검증에서 모든 내장 asset을 실제 렌더해 오류를 잡고, 설치 환경에서 필요한 기존 SVG loader 제공 여부를 확인한다. API signature는 현재 로컬 crate에 맞춰 사용한다.
- 기존 window close/minimize/maximize 동작과 접근성 이름, tooltip, 36px 클릭 영역을 유지한다. 단순 이미지 변경으로 저장/close coordinator를 건드리지 않는다.

## 완료 증거와 독립 리뷰

1. 실제 light/dark 목록+편집 창을 각각 촬영한다. 모든 창 조작·palette·more·add가 같은 선굵기, 크기, 모서리 문법이며 컬러 원/문자 glyph/system palette가 남지 않으면 통과.
2. 최대화 전/후 restore 아이콘 전환, close hover, 일반 hover, keyboard focus를 촬영 또는 실행 확인한다. 아이콘 위치 흔들림·버튼 크기 변화·기능/tooltip 불일치는 실패.
3. 좁은 폭에서 기존 제목/버튼 겹침이 없고, 1× 및 가능한 2×에서 선이 읽히는지 확인한다. 2×를 실행하지 않았다면 검증 완료로 보고하지 않는다.
4. 새 SVG 전체 decode smoke check와 fmt/build/clippy를 수행한다. 데이터 로직 변경은 없어 새 저장 테스트는 만들지 않되 기존 테스트는 프로젝트 완료 검증에서 유지한다.
5. Astra는 실제 screenshot을 독립 리뷰한다. 전체 UI를 다시 설계하거나 새로운 기능으로 범위를 넓히지 않는다.
