<p align="center">
  <img src="integration/memoivy.svg" width="80" height="80" alt="">
</p>

<h1 align="center">MemoIvy</h1>

<p align="center"><strong>작은 메모에서, 연결된 생각으로.</strong></p>
<p align="center">적고 · 묶고 · 연결하는 Linux 데스크톱 메모 앱</p>
<p align="center">Rust · GTK 4 · MIT License</p>

---

떠오르는 생각은 메모로 남기고, 관련된 메모는 프로젝트로 묶으세요.<br>
캔버스 위에서 생각을 연결하고, 체크리스트로 다음 할 일을 이어 갑니다.

![색상별 메모 네 개를 캔버스에 배치하고 아이디어 기록, 생각 연결, 작업 완료 순서로 연결한 실제 MemoIvy 화면](docs/assets/canvas.png)

*현재 개발 버전의 실제 화면입니다. 예시 메모는 합성 데이터입니다.*

## 생각을 정리하는 세 가지 방법

### 01 · 가볍게 적기

독립된 메모 창에서 Markdown 서식을 보며 작성합니다. 색상을 바꾸고, 체크리스트와 이미지·파일을 넣고, 자동 저장으로 기록을 이어 갑니다.

### 02 · 한곳에 모으기

목록에서 검색하고 긴 본문을 바로 펼쳐 읽습니다. 관련 메모를 프로젝트 안에 묶고, 하위 프로젝트로 나누어 정리합니다.

<img src="docs/assets/library.png" width="360" alt="검색창 아래 노랑·파랑·초록·분홍 메모가 보이는 목록. 체크리스트 진행률과 본문 펼침 버튼을 함께 표시합니다.">

### 03 · 흐름으로 연결하기

캔버스에서 카드를 끌어 배치하고 다음 메모를 연결합니다. 여러 카드를 선택해 정렬하거나 간격을 맞추고, 배치·연결·완료 변경을 되돌릴 수 있습니다. 카드를 두 번 누르면 원본 메모가 열립니다.

## 시작하기

Linux 데스크톱에서 사용할 수 있습니다. 소스 빌드에는 **GTK 4.10 이상**과 **mise**가 필요합니다.

```bash
# Ubuntu 시스템 패키지와 Rust 준비 (시스템 패키지 설치에 sudo 필요)
mise bootstrap

# 개발 버전 실행
mise run dev
```

앱 메뉴와 바탕화면에 설치하거나 기존 설치본을 갱신하려면:

```bash
mise run install
```

실행 중인 MemoIvy는 종료한 뒤 다시 열어 주세요.

## 손에 익으면 더 빠르게

| 하고 싶은 일 | 조작 |
| --- | --- |
| 앱·서식 단축키 확인 | `F1` |
| 체크 항목 완료·해제 | 체크박스 클릭 또는 `Ctrl+Enter` |
| 링크 열기 | `Ctrl+클릭` · 마우스를 올리면 실제 주소 표시 |
| Markdown 원문 전환 | `Ctrl+Shift+M` 또는 더보기 메뉴 |
| 캔버스에서 여러 카드 선택 | 빈 곳에서 끌기 |
| 선택한 캔버스 메모 삭제 | `Delete` → 확인 후 삭제 |

## 개발·검증

| 작업 | 명령 |
| --- | --- |
| 포맷·전체 테스트·Clippy | `mise run check` |
| 릴리스 빌드 | `mise run build` |
| Snap 패키지 빌드 | `mise run package:snap` |
| 커밋·푸시 비밀정보 검사 활성화 | `mise install` 후 `mise run hooks:install` |

상세 안내는 `docs`에서 **설치**(OPS-001), **백업·복구**(OPS-002), **개발 검증**(OPS-003), **아키텍처** 관심사로 찾을 수 있습니다.

[버그 제보·기능 제안](https://github.com/pollux-o4/MemoIvy/issues) · [MIT License](LICENSE)
