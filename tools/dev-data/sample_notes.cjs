'use strict';
// 개발용 표본 메모: 캔버스·목록·프로젝트 화면의 경우의 수를 한 데이터에 모은다.
// 결정적 생성(같은 입력 → 같은 UUID·좌표)이라 화면 비교·버그 재현에 다시 쓸 수 있다.

const COLORS = ['yellow', 'pink', 'blue', 'green', 'purple'];
const BASE_TIME = 1794576000;

function createBuilder() {
  const notes = [];
  let seq = 0;
  const id = () => `d0000000-0000-4000-8000-${(++seq).toString(16).padStart(12, '0')}`;
  /** 메모 하나를 더하고 id를 돌려준다. position null은 자동 배치 경우. */
  const add = (text, { color = COLORS[seq % COLORS.length], position = null, next = [], done = false, project = false, parent = null, pinned = false } = {}) => {
    const noteId = id();
    const canvas = { position, next, done };
    if (project) canvas.project = true;
    if (parent) canvas.parent = parent;
    if (pinned) canvas.pinned = true;
    notes.push({ id: noteId, text, color, modified: BASE_TIME - notes.length * 3600, canvas });
    return noteId;
  };
  /** 만든 뒤에 연결을 잇는다(순환·역방향 연결용). */
  const link = (from, to) => notes.find((note) => note.id === from).canvas.next.push(to);
  return { notes, add, link };
}

/** 색·완료·본문 형식(마크다운·체크리스트·빈 메모·긴 글). */
function contentCases({ add }) {
  COLORS.forEach((color, i) => { add(`${color} 카드\n색상 확인용 메모입니다.`, { color, position: [80 + i * 280, -420], done: i % 2 === 1 }); });
  add('', { position: [80, -160] });
  add('제목만 있는 메모', { position: [360, -160] });
  add(`${'아주 긴 제목이 카드 폭을 넘어갈 때 줄바꿈과 말줄임을 확인합니다 '.repeat(2)}\n본문`, { position: [640, -160] });
  add(['체크리스트 진행', '- [x] 끝난 일', '- [ ] 남은 일', '  - [ ] 하위 할 일', '  - [x] 끝난 하위 할 일', '- [ ] 또 남은 일'].join('\n'), { color: 'green', position: [920, -160] });
  add(['마크다운 모음', '## 소제목', '- 글머리 하나', '  - 들여쓴 글머리', '1. 번호 목록', '> 인용문', '`인라인 코드`와 **굵게**, *기울임*', '[링크](https://example.com)', '```', 'const code = true;', '```'].join('\n'), { color: 'purple', position: [1200, -160] });
  add(`긴 본문\n${'여러 줄에 걸친 긴 본문이 미리보기에서 잘리는지, 펼치기가 나타나는지 확인합니다. '.repeat(12)}`, { color: 'blue', position: [1480, -160] });
}

/** 연결 모양: 마인드맵 트리, 직선 사슬, 한→여럿, 여럿→한, 순환. */
function linkCases({ add, link }) {
  const leaf = (text, x, y, done = false) => add(text, { position: [x, y], done });
  const g = [leaf('손자 1-1', 1040, 200, true), leaf('손자 1-2', 1040, 380)];
  const c1 = add('자식 1', { position: [760, 290], next: g });
  const c2 = add('자식 2', { position: [760, 470], next: [leaf('손자 2-1', 1040, 560)] });
  const c3 = add('자식 3 (완료)', { position: [760, 650], done: true });
  add('마인드맵 뿌리\n가지 개요·Tab 가지 확인', { color: 'yellow', position: [480, 470], next: [c1, c2, c3] });

  let next = [];
  for (let i = 4; i >= 1; i -= 1) next = [add(`사슬 ${i}`, { position: [80 + (i - 1) * 280, 900], next })];

  const fanOut = Array.from({ length: 5 }, (_, i) => add(`갈래 ${i + 1}`, { position: [1520, 200 + i * 200] }));
  add('한 곳에서 여러 갈래로', { color: 'pink', position: [1240, 600], next: fanOut });
  const sink = add('여러 곳에서 모이는 메모', { color: 'green', position: [2120, 600] });
  for (const from of fanOut.slice(0, 4)) link(from, sink);

  const a = add('순환 A', { position: [80, 1200] });
  const b = add('순환 B', { position: [360, 1200], next: [] });
  const c = add('순환 C', { position: [220, 1420], next: [a] });
  link(a, b); link(b, c);
}

/** 위치: 음수 좌표, 겹침, 먼 카드, 자동 배치(위치 없음). */
function placementCases({ add }) {
  add('원점 위·왼쪽 (음수 좌표)', { position: [-600, -700] });
  add('겹친 카드 아래', { position: [-600, 200] });
  add('겹친 카드 위', { position: [-540, 240] });
  add('저장 범위 끝 근처의 먼 카드', { position: [19_000, 19_000] });
  for (const text of ['자동 배치 1', '자동 배치 2', '자동 배치 3']) add(text);
}

/** 프로젝트: 빈 것, 고정, 완료, 묶음 많은 것(더 보기), 3단 중첩, 대량(성능). */
function projectCases({ add }) {
  add('빈 프로젝트', { project: true, position: [2400, -420] });

  const pinned = add('고정한 프로젝트\n바로가기에 보입니다.', { project: true, pinned: true, color: 'blue', position: [2400, -160] });
  let next = [];
  for (let i = 3; i >= 1; i -= 1) next = [add(`고정 프로젝트 할 일 ${i}`, { parent: pinned, position: [80 + (i - 1) * 280, 80], done: i === 1, next })];

  const done = add('완료한 프로젝트', { project: true, done: true, color: 'green', position: [2400, 100] });
  add('끝난 일', { parent: done, position: [80, 80], done: true });

  const many = add('메모가 많은 프로젝트\n펼침 미리보기의 "더 보기"를 확인합니다.', { project: true, color: 'pink', position: [2400, 360] });
  for (let i = 0; i < 14; i += 1) add(`많은 프로젝트 메모 ${i + 1}`, { parent: many, position: [80 + (i % 5) * 280, 80 + Math.floor(i / 5) * 220], done: i % 4 === 0 });

  let parent = add('중첩 프로젝트 1단', { project: true, color: 'purple', position: [2400, 620] });
  for (let depth = 2; depth <= 3; depth += 1) {
    add(`${depth - 1}단 안의 메모`, { parent, position: [80, 80] });
    parent = add(`중첩 프로젝트 ${depth}단`, { project: true, parent, color: 'purple', position: [360, 80] });
  }
  add('가장 안쪽 메모', { parent, position: [80, 80] });

  const bulk = add('대량 프로젝트 (200장)\n성능·스크롤 확인용', { project: true, color: 'yellow', position: [2400, 880] });
  for (let i = 0; i < 200; i += 1) add(`대량 메모 ${i + 1}`, { parent: bulk, position: [80 + (i % 20) * 280, 80 + Math.floor(i / 20) * 220], done: i % 7 === 0, color: COLORS[i % COLORS.length] });
}

/** 연결 끝이 꾸밈에 묻히기 쉬운 자리: 프로젝트 아래·오른쪽(종이 더미), 펼친 긴 본문 카드 아래. */
function linkEdgeCases({ add }) {
  const project = add('연결이 들어오는 프로젝트', { project: true, color: 'pink', position: [3000, -420] });
  add('끝난 하위 메모', { parent: project, position: [80, 80], done: true });
  add('프로젝트 아래에서 위로', { position: [3000, -40], next: [project] });
  add('프로젝트 오른쪽에서 왼쪽으로', { position: [3500, -420], next: [project] });
  const long = add(['펼치면 길어지는 메모', ...Array.from({ length: 8 }, (_, i) => `- [ ] 펼친 미리보기 줄 ${i + 1}`)].join('\n'), { color: 'blue', position: [3000, 300] });
  add('긴 메모 아래에서 위로', { position: [3000, 900], next: [long] });
}

const CASES = { contentCases, linkCases, placementCases, projectCases, linkEdgeCases };

/** names로 고른 경우 묶음만(테스트가 한 화면에 담을 때), 없으면 전부. */
function sampleNotes(names = Object.keys(CASES)) {
  const builder = createBuilder();
  for (const name of names) CASES[name](builder);
  return builder.notes;
}

module.exports = { sampleNotes };
