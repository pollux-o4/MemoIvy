/** GTK shared/shortcuts.rs 안내 데이터의 단일 출처: [뜻, Markdown 문법, 키들]. */
export type ShortcutRow = readonly [meaning: string, syntax: string, keys: readonly string[]];
export type ShortcutSection = readonly [title: string, rows: readonly ShortcutRow[]];

export const NOTE_SHORTCUTS: readonly ShortcutSection[] = [
  ['앱', [['새 메모', '', ['Ctrl+N']], ['메모 목록 검색', '', ['Ctrl+F']], ['현재 창 닫기', '', ['Ctrl+W']], ['이 안내', '', ['F1']]]],
  ['메모 서식', [
    ['제목', '# 제목', ['Ctrl+1']], ['소제목', '## 소제목', ['Ctrl+2']], ['굵게', '**글자**', ['Ctrl+B']], ['기울임', '*글자*', ['Ctrl+I']],
    ['취소선', '~~글자~~', ['Ctrl+Shift+X']], ['링크', '[이름](주소)', ['Ctrl+K']], ['번호 목록', '1. 항목', ['Ctrl+Shift+7']],
    ['글머리표', '- 항목', ['Ctrl+Shift+8']], ['체크 목록', '- [ ] 할 일', ['Ctrl+Shift+9']],
  ]],
  ['메모 편집', [['체크 변경 · 링크 열기', '', ['Ctrl+Enter']], ['원문 보기', '', ['Ctrl+Shift+M']], ['실행 취소', '', ['Ctrl+Z']], ['다시 실행', '', ['Ctrl+Shift+Z']]]],
];

export const CANVAS_SHORTCUTS: readonly ShortcutSection[] = [
  ['캔버스 선택', [
    ['메모 열기', '', ['두 번 누르기', 'Enter']], ['선택에 더하기', '', ['Shift+누르기']], ['모두 선택', '', ['Ctrl+A']], ['선택 해제', '', ['Esc']],
    ['선택한 메모 삭제', '', ['Delete', 'Backspace']], ['완료 표시', '', ['Ctrl+Enter']], ['다음 메모 연결', '', ['Ctrl+L']], ['복사 · 잘라내기', '', ['Ctrl+C', 'Ctrl+X']], ['붙여넣기', '', ['Ctrl+V']], ['복제', '', ['Ctrl+D']], ['이름 바꾸기', '', ['F2']], ['카드 메뉴', '', ['오른쪽 클릭', 'Shift+F10']], ['이동', '', ['방향키']], ['크게 이동', '', ['Shift+방향키']],
  ]],
  ['캔버스 보기', [['전체 보기', '', ['Shift+1']], ['선택 영역 보기', '', ['Shift+2']], ['확대 · 축소', '', ['Ctrl++', 'Ctrl+-']], ['100%로 보기', '', ['Ctrl+0']], ['보드 이동', '', ['Space+끌기']]]],
  ['프로젝트', [['프로젝트 들어가기', '', ['두 번 누르기', 'Enter']], ['프로젝트 설명 열기', '', ['Shift+Enter']], ['밖으로 나가기', '', ['Esc', 'Alt+↑']], ['새 프로젝트로 묶기', '', ['Alt+G']], ['밖으로 꺼내기', '', ['Alt+Shift+G']]]],
  ['캔버스 정렬', [
    ['왼쪽 · 가운데 · 오른쪽', '', ['Alt+A', 'Alt+H', 'Alt+D']], ['위쪽 · 가운데 · 아래쪽', '', ['Alt+W', 'Alt+V', 'Alt+S']],
    ['간격 같게 (가로 · 세로)', '', ['Alt+Shift+H', 'Alt+Shift+V']], ['격자로 정리', '', ['Alt+T']], ['연결 순서로 배치', '', ['Alt+F']],
  ]],
];
