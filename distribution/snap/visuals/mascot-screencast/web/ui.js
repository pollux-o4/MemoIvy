// MemoIvy 창을 상태 → HTML로 다시 그린다. 문구·색은 frontend/shared/tokens.css와 Storybook 'screens-window-states'의 실제 값.
// 좌표는 월드(1920×1080) 기준, data-a = 마스코트·커서·카메라가 재는 앵커.

const I = {
  close: '<path d="M6 6l12 12M18 6L6 18"/>', min: '<path d="M6 12h12"/>', max: '<rect x="6" y="6" width="12" height="12" rx="2"/>',
  graph: '<circle cx="6" cy="12" r="2.4"/><circle cx="17" cy="6" r="2.4"/><circle cx="17" cy="18" r="2.4"/><path d="M8.2 11l6.6-3.8M8.2 13l6.6 3.8"/>',
  tune: '<path d="M4 8h9M17 8h3M4 16h3M11 16h9"/><circle cx="15" cy="8" r="2"/><circle cx="9" cy="16" r="2"/>',
  plus: '<path d="M12 5v14M5 12h14"/>', search: '<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>',
  chev: '<path d="M7 10l5 5 5-5"/>', chevUp: '<path d="M7 14l5-5 5 5"/>', palette: '<circle cx="12" cy="12" r="8"/><circle cx="9" cy="10" r="1"/><circle cx="14" cy="9" r="1"/><circle cx="15.5" cy="13" r="1"/>',
  more: '<circle cx="6" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="18" cy="12" r="1.2"/>',
  undo: '<path d="M9 8L5 12l4 4M5 12h9a5 5 0 0 1 0 10"/>', redo: '<path d="M15 8l4 4-4 4M19 12h-9a5 5 0 0 0 0 10"/>', fit: '<rect x="5" y="5" width="14" height="14" rx="3"/><rect x="9" y="9" width="6" height="6" rx="1"/>',
  help: '<circle cx="12" cy="12" r="8"/><path d="M10 10a2 2 0 1 1 3 1.7c-.7.4-1 .8-1 1.6M12 16.5v.1"/>', link: '<circle cx="7" cy="17" r="2"/><circle cx="17" cy="7" r="2"/><path d="M8.5 15.5l7-7"/>',
  compass: '<circle cx="12" cy="12" r="8"/><path d="M15 9l-2 4-4 2 2-4z"/>',
  home: '<path d="M5 11l7-6 7 6v8H5z"/>',
  folder: '<path d="M4 7a1 1 0 0 1 1-1h4l2 2h8a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"/>',
  tick: '<path d="M6.5 12.5l3.5 3.5 7.5-8"/>', arrow: '<path d="M8 16L16 8M10 8h6v6"/>',
};
export const icon = (k, s = 22, extra = '') => `<svg class="ic" viewBox="0 0 24 24" width="${s}" height="${s}" ${extra}>${I[k]}</svg>`;
export const NOTE = { yellow: ['#fff3c4', '#e9bf3f'], pink: ['#fde2e9', '#ec86a2'], blue: ['#dcebfd', '#5fa3ee'], green: ['#e0f2dc', '#72bd6d'], purple: ['#ebe3fa', '#a386e0'] };

// 창 틀: 헤더(닫기·최소화·최대화 · 제목 · 오른쪽 도구)
export function win({ id, x, y, w, h, title, right = '', bg = 'var(--bg)', body, op = 1, k = 1, z = 1 }) {
  return `<section class="win" data-a="${id}" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;background:${bg};opacity:${op};transform:scale(${k});z-index:${z}">
    <header><span class="wc">${icon('close')}${icon('min')}${icon('max')}</span><b data-a="${id}-title">${title}</b><span class="wr">${right}</span></header>${body}</section>`;
}
const newBtn = id => `<span class="new" data-a="${id}">${icon('plus', 20)}New</span>`;
const checkbox = (on, a = '') => `<i class="cb ${on ? 'on' : ''}" ${a ? `data-a="${a}"` : ''}>${on ? icon('tick', 16) : ''}</i>`;

// ── 목록 창 ──
export const LIB = { x: 680, y: 110, w: 560, h: 860 };
// 1막 스티키 메모와 같은 내용(비포·애프터 비교)
export const MEMOS = [
  { id: 'trip', color: 'yellow', title: '주말 여행 준비', items: [['기차표 예매', 1], ['숙소 전화', 1], ['우산 챙기기', 0]] },
  { id: 'meet', color: 'blue', title: '회의 3시', body: '분기 계획 공유, 자료는 전날 정리해 두기.' },
  { id: 'photo', color: 'pink', title: '여행 사진 정리', body: '바다 사진은 따로 묶고, 마음에 드는 열 장만 남기기.' },
  { id: 'shop', color: 'green', title: '장보기', body: '우유, 달걀, 사과, 커피 원두.' },
  { id: 'tax', color: 'purple', title: '세금 납부', body: '이번 달 25일까지, 자동이체 확인.' },
];
// 검색 규칙(SSoT): 제목·본문·할 일 중 하나에 검색어가 있으면 결과
export const matches = (m, q) => !q || [m.title, m.body || '', ...(m.items || []).map(i => i[0])].some(x => x.includes(q));
// 카드의 등장(enter: 제자리 페이드)과 크기(size: 목록에서 접혀 사라짐)는 독립 축
const presence = ({ enter = 1, size = 1 }) => size >= 1
  ? `opacity:${enter};transform:translateY(${(1 - enter) * -18}px)`
  : `opacity:${enter * size};max-height:${size * 220}px;margin-bottom:${size * 10}px;padding-top:${size * 14}px;padding-bottom:${size * 14}px;overflow:hidden`;
const mark = (t, q) => (q && t.includes(q) ? t.replace(q, `<mark>${q}</mark>`) : t);
function memoCard(m, { expand = 0, enter = 1, size = 1, hl = '' } = {}) {
  const done = m.items ? m.items.filter(i => i[1]).length : 0;
  const item = ([t, d], extra = 1) => `<div class="ci ${d ? 'dn' : ''}" style="${extra < 1 ? `opacity:${extra};max-height:${extra * 40}px;overflow:hidden` : ''}">${checkbox(d)}<span>${mark(t, hl)}</span></div>`;
  const inner = m.items
    ? `<div class="mt"><b>${m.title}</b><span class="badge">${done}/${m.items.length}</span></div><div class="prog"><i style="width:${done / m.items.length * 100}%"></i></div>
       ${m.items.slice(0, 2).map(i => item(i)).join('')}${expand > 0 ? m.items.slice(2).map(i => item(i, expand)).join('') : ''}
       <div class="exp" data-a="exp-${m.id}">${icon(expand > .5 ? 'chevUp' : 'chev', 20)}</div>`
    : `<div class="mt"><b>${m.title}</b></div><p>${mark(m.body, hl)}</p>`;
  return `<article class="memo" data-a="memo-${m.id}" style="--dot:${NOTE[m.color][1]};${presence({ enter, size })}">${inner}<span class="mm">${icon('more', 20)}</span></article>`;
}
// 목록 창. typed: 입력란 글자, query: 거를 검색어, filter: 거르기 진행(0→1), enter[i]: 카드별 등장, expand: 결과 카드 펼침
export function library({ typed = '', query = '', filter = 0, enter = null, expand = 0, caret = false, focus = false, op = 1, k = 1, z = 1 } = {}) {
  const hits = MEMOS.filter(m => matches(m, query)).length, shown = enter ? enter.filter(e => e > .5).length : MEMOS.length;
  const cards = MEMOS.map((m, i) => { const hit = matches(m, query); return hit || filter < 1 ? memoCard(m, { enter: enter ? enter[i] : 1, size: hit ? 1 : 1 - filter, expand: hit ? expand : 0, hl: filter > 0 ? query : '' }) : ''; }).join('');
  const count = filter > 0 ? `검색 결과 ${hits}개` : `메모 ${shown}개`;
  const body = `<div class="lib"><div class="srch ${focus ? 'on' : ''}" data-a="search">${icon('search', 20)}<span class="${typed ? '' : 'ph'}">${typed || '메모 검색'}${caret ? '<i class="caret"></i>' : ''}</span></div>
    <div class="cnt pv"><span>${count}</span>${filter > 0 ? '' : '<span class="seg"><i>프로젝트별</i><b>최근</b></span>'}</div>${cards}</div>`;
  return win({ id: 'lib', ...LIB, title: 'MemoIvy', right: `<span data-a="canvasBtn">${icon('graph')}</span>${icon('tune')}${newBtn('new')}`, body, op, k, z });
}

// ── 메모 창(독립 편집 창, 노란 메모) ──
export const ED = { x: 790, y: 170, w: 520, h: 620 };
export const ED_LINES = [['주말 여행 준비', 'h'], ['기차표 예매', 'c'], ['숙소 확인', 'c'], ['우산 챙기기', 'c']];
// cnt: 줄마다 입력된 글자 수, cur: 캐럿이 있는 줄(-1이면 숨김), checked: 완료한 체크 항목 수, save: 하단 저장 상태
export function editor({ cnt = ED_LINES.map(l => l[0].length), cur = -1, checked = 0, save = '저장됨', op = 1, k = 1, z = 2 } = {}) {
  let rows = '';
  ED_LINES.forEach(([text, kind], i) => {
    if (i > 0 && cnt[i] === 0 && cur < i) return;
    const caret = cur === i ? '<i class="caret" data-a="caret"></i>' : '', done = kind === 'c' && i <= checked;
    const span = `<span data-a="ed-t${i}">${text.slice(0, cnt[i])}</span>${caret}`;
    rows += kind === 'h' ? `<div class="eh" data-a="ed-l${i}">${span}</div>`
      : `<div class="ec ${done ? 'dn' : ''}" data-a="ed-l${i}">${checkbox(done, `ed-cb${i}`)}${span}</div>`;
  });
  const body = `<div class="ed">${rows}</div><div class="save" data-a="save">${save}</div>`;
  return win({ id: 'ed', ...ED, title: '메모', bg: NOTE.yellow[0], right: `${icon('graph')}${icon('palette')}${icon('more')}`, body, op, k, z });
}

// ── 캔버스 창 ──
export const CV = { x: 130, y: 80, w: 1660, h: 920 };
export const CARDS = {
  trip: { x: 150, y: 250, w: 330, h: 230, color: 'yellow', title: '주말 여행 준비', items: MEMOS[0].items },
  train: { x: 640, y: 120, w: 310, h: 170, color: 'blue', title: '기차표 예매', body: '토요일 09:10 출발, 창가 자리 두 장.' },
  stay: { x: 640, y: 470, w: 310, h: 170, color: 'green', title: '숙소 전화', body: '체크인 15시, 주차 되는지 물어보기.' },
  photo: { x: 1110, y: 290, w: 330, h: 190, color: 'pink', title: '여행 사진 정리', body: '바다 사진은 따로 묶고, 마음에 드는 열 장만 남기기.' },
};
// 연결 곡선: 출발 카드 오른쪽 가운데 → 도착 카드 왼쪽 가운데(캔버스 지역 좌표, 헤더 아래 기준)
export function linkPath(a, b) {
  const A = CARDS[a], B = CARDS[b], p0 = [A.x + A.w, A.y + A.h / 2], p3 = [B.x, B.y + B.h / 2], dx = (p3[0] - p0[0]) * .5;
  return [p0, [p0[0] + dx, p0[1]], [p3[0] - dx, p3[1]], p3];
}
export function bez(P, s) {
  const u = 1 - s; return [0, 1].map(i => u * u * u * P[0][i] + 3 * u * u * s * P[1][i] + 3 * u * s * s * P[2][i] + s * s * s * P[3][i]);
}
export const HEAD = 56;   // 창 헤더 높이: 캔버스 지역 좌표 → 월드 좌표 변환에 쓴다
export const cvWorld = ([x, y]) => [CV.x + x, CV.y + HEAD + y];

function card(id, c, { done = 0, complete = false, hot = false } = {}) {
  const [bg, dot] = NOTE[c.color];
  const inner = c.items
    ? `${c.items.map(([t], i) => `<div class="ci ${i < done ? 'dn' : ''}">${checkbox(i < done, `cv-cb${i}`)}<span>${t}</span></div>`).join('')}
       <div class="cprog"><span>할 일 ${done}/${c.items.length}</span><div class="prog"><i style="width:${done / c.items.length * 100}%"></i></div></div>`
    : `<p>${c.body}</p>`;
  return `<div class="card ${hot ? 'hot' : ''}" data-a="card-${id}" style="left:${c.x}px;top:${c.y}px;width:${c.w}px;height:${c.h}px;background:${bg};border-color:${dot}">
    <div class="mt"><b data-a="ct-${id}">${c.title}</b>${complete ? `<span class="okb" data-a="okb">${icon('tick', 18)}</span>` : ''}</div>${inner}</div>`;
}
// ── 전체(루트) 캔버스: 프로젝트는 종이 더미 카드, 하위 메모는 그 안의 서브그래프(실제 앱 ProjectFan·CardHeader) ──
export const ROOT = {
  trip: { x: 600, y: 300, w: 380, h: 210, color: 'yellow', title: '주말 여행' },
  loose: [['meet', 1150, 150, 'blue', '회의 3시'], ['shop', 1180, 470, 'green', '장보기'], ['tax', 180, 520, 'purple', '세금 납부']],
};
// 부채: 최근 하위 메모 3장 제목 + 나머지 수(project_fan.ts FAN_LIMIT = 3)
const FAN = { titles: [['photo', '여행 사진 정리'], ['stay', '숙소 전화'], ['train', '기차표 예매']], more: 1 };
const SLOT = [[-3, -36, -1], [10, -72, 2], [-8, -108, -2.5]], REST = [[5, 5, .6], [10, 10, 1.2], [10, 10, 1.2]];
function projectStack({ fan = 0, done = 2, all = 3, n = 4, hot = false }) {
  const P = ROOT.trip, [bg, dot] = NOTE[P.color];
  const sheets = FAN.titles.map(([c, t], i) => {
    const [rx, ry, rr] = REST[i], [fx, fy, fr] = SLOT[i], x = rx + (fx - rx) * fan, y = ry + (fy - ry) * fan, r = rr + (fr - rr) * fan;
    return `<div class="sheet" style="left:${P.x}px;top:${P.y}px;width:${P.w}px;height:${P.h}px;background:${bg};opacity:${i < 2 ? 1 : fan};transform:translate(${x}px,${y}px) rotate(${r}deg);z-index:${3 - i}">
      <span class="st" style="opacity:${fan}"><i style="background:${NOTE[CARDS[c].color][1]}"></i>${t}</span>${i === 2 && FAN.more ? `<small style="opacity:${fan}">외 ${FAN.more}개</small>` : ''}</div>`;
  }).reverse().join('');
  const ok = done >= all;
  return sheets + `<div class="card pcard ${hot ? 'hot' : ''}" data-a="root-trip" style="left:${P.x}px;top:${P.y}px;width:${P.w}px;height:${P.h}px;background:${bg};border-color:${dot}">
    <div class="mt"><b>${icon('folder', 22)}${P.title}</b><span class="pact">${icon('arrow', 18)}<i class="ring ${ok ? 'on' : ''}">${ok ? icon('tick', 14) : ''}</i></span></div>
    <p>여행 준비 메모 모음</p><div class="pfoot"><b>메모 ${n}개</b><span>완료 ${done}/${all}</span></div></div>`;
}
// inside: 0 = 전체, 1 = 프로젝트 안(서브그래프). 들어갈 때 더미 카드가 커지며 사라지고 안쪽 카드가 떠오른다.
// links: [{a, b, k}] k = 그려진 비율, drag: 끌고 있는 선 끝(캔버스 지역 좌표)
export function canvas({ inside = 1, fan = 0, links = [], drag = null, done = 2, complete = false, hot = null, save = '저장됨', op = 1, k = 1, z = 3 } = {}) {
  const P = ROOT.trip, ox = P.x + P.w / 2, oy = P.y + P.h / 2;
  const root = inside < 1 ? `<div class="layer" style="opacity:${1 - inside};transform-origin:${ox}px ${oy}px;transform:scale(${1 + inside * .8})">
      ${ROOT.loose.map(([id, x, y, c, t]) => `<div class="card mini" style="left:${x}px;top:${y}px;background:${NOTE[c][0]};border-color:${NOTE[c][1]}"><div class="mt"><b>${t}</b></div></div>`).join('')}
      ${projectStack({ fan, done, hot: hot === 'root' })}</div>` : '';
  let sub = '';
  if (inside > 0) {
    const paths = links.map(({ a, b, k }) => {
      const [p0, c1, c2, p3] = linkPath(a, b);
      return `<path d="M${p0}C${c1} ${c2} ${p3}" pathLength="1" stroke-dasharray="${k} 1" marker-end="${k >= .98 ? 'url(#arr)' : ''}"/>`;
    }).join('') + (drag ? `<path class="dg" d="M${drag.from}C${drag.from[0] + 120},${drag.from[1]} ${drag.to[0] - 120},${drag.to[1]} ${drag.to}"/><circle cx="${drag.to[0]}" cy="${drag.to[1]}" r="7" class="dgp"/>` : '');
    const cards = Object.entries(CARDS).map(([id, c]) => card(id, c, { done: id === 'trip' ? done : 0, complete: id === 'trip' && complete, hot: hot === id })).join('');
    sub = `<div class="layer" style="opacity:${inside};transform-origin:${ox}px ${oy}px;transform:scale(${.7 + .3 * inside})"><svg class="links" width="${CV.w}" height="${CV.h - HEAD}"><defs><marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="9" markerHeight="9" orient="auto"><path d="M1 1L9 5L1 9" fill="none" stroke="#6b6e75" stroke-width="1.6"/></marker></defs>${paths}</svg>${cards}</div>`;
  }
  const deep = inside >= .5, n = links.filter(l => l.k >= .98).length;
  const crumb = `<div class="crumb" data-a="crumb"><span class="mu" data-a="crumb-home">${icon('home', 18)} 전체</span>${deep ? '<span class="mu">›</span><b>주말 여행</b><span class="star">★</span>' : ''}</div>`;
  const foot = deep ? `메모 4 · 연결 ${n}` : '메모 8 · 프로젝트 1';
  const body = `<div class="cv">${root}${sub}${crumb}
    <div class="tb">${icon('undo')}${icon('redo')}<i class="sep"></i><b>−</b><span>100%</span><b>+</b>${icon('fit')}<i class="sep"></i>${icon('help')}<span class="mu" data-a="cvfoot">${foot}</span><span class="mu">${save}</span></div></div>`;
  return win({ id: 'cv', ...CV, title: deep ? '주말 여행' : 'MemoIvy', right: `${icon('tune')}${newBtn('cvnew')}`, body, op, k, z });
}

// ── 독(바탕화면) ──
export const DOCK = { x: 960, y: 960 };
// leaf: 아이콘 위쪽 잎이 붙어 있는지(떨어지면 잎새가 된다), wob: 떨어지기 전 흔들림(라디안)
export function dock({ leaf = true, wob = 0, hot = false } = {}) {
  return `<div class="dock" data-a="dockbar" style="left:${DOCK.x}px;top:${DOCK.y}px"><i class="di" style="background:#8fa4c0"></i><i class="di" style="background:#e3b86b"></i>
    <span class="dapp ${hot ? 'hot' : ''}" data-a="dockicon"><img src="assets/${leaf ? 'memoivy' : 'memoivy-noleaf'}.svg" style="transform:rotate(${wob}rad)"></span><i class="di" style="background:#9b8fd0"></i><i class="di" style="background:#7cb7a3"></i></div>`;
}

// ── 1막: 평범한 종이 스티키 메모(특정 제품이 아닌 일반 노란 메모). 좌표는 왼쪽 위, r은 기울기(도)
export const STICKIES = [
  ['s1', 250, 150, -3, '기차표 예매하기'], ['s2', 500, 230, 4, '회의 3시'], ['s3', 760, 140, -2, '우산!!'], ['s4', 1010, 220, 3, '사진 정리'],
  ['s5', 1250, 160, -4, '숙소 전화'], ['s6', 1480, 240, 2, '장보기: 우유, 달걀'], ['s7', 380, 520, 2, '주말 여행 준비'], ['s8', 650, 560, -3, '택배 반품'],
  ['s9', 930, 590, 4, '책 반납'], ['s10', 1200, 520, -2, '운동 7시'], ['s11', 1440, 590, 3, '생일 선물'],
  ['s12', 1230, 210, -7, '치과 예약'], ['s13', 560, 330, 5, '세금 납부'], ['s14', 960, 400, -4, '전화 회신'], ['s15', 930, 430, 6, '할 일 또..'],
];
export const STICKY_COL = ['#fff0a8', '#ffe98a', '#fff4bd', '#ffe27a'];
// at: 떨어지는 시각(없으면 처음부터), off: 끌어서 옮긴 거리 [dx, dy]
export function stickies(t, { at = {}, off = {} } = {}) {
  return STICKIES.map(([id, x, y, r, text], i) => {
    const t0 = at[id]; if (t0 !== undefined && t < t0 - .12) return '';
    const k = t0 === undefined ? 1 : Math.min(1, (t - t0 + .12) / .12), sc = 1.3 - .3 * k, [dx, dy] = off[id] || [0, 0];
    return `<div class="sticky" data-a="st-${id}" style="left:${x + dx}px;top:${y + dy}px;transform:rotate(${r}deg) scale(${sc});opacity:${Math.min(1, k * 1.5)};background:${STICKY_COL[i % 4]}">${text}</div>`;
  }).join('');
}

// ── 쓰기 장면: 실제 앱 메모 창(capture/shots/05·08·09)을 따른다. FMT-001·003·004
export const RICH = { x: 1040, y: 250, w: 640, h: 740 };
// head·bold: 서식 적용(커서가 떠난 줄), half: 진행 중, link/img: 0..1 나타남, caret: 커서 줄
export function rich({ head = false, bold = false, half = false, link = 0, pop = 0, img = 0, caret = -1, op = 1, k = 1, z = 4 } = {}) {
  const c = i => (caret === i ? '<i class="caret"></i>' : ''), raw = x => `<span class="md">${x}</span>`;
  const box = (st, a = '') => st === 'on' ? checkbox(true, a) : st === 'half' ? `<i class="cb half" data-a="${a}"><i></i></i>` : checkbox(false, a);
  const rows = [
    `<div class="eh">${head ? '' : raw('# ')}주말 여행 준비${c(0)}</div>`,
    `<div class="ec dn">${box('on')}<span>기차표 예매</span></div>`,
    `<div class="ec dn">${box('on')}<span>숙소 전화</span></div>`,
    `<div class="ec">${box(half ? 'half' : 'off', 'rcb3')}<span>짐 싸기</span></div>`,
    `<div class="ec">${box('off')}<span>우산 챙기기</span></div>`,
    `<div class="ep">${bold ? '<b>토요일 09:10</b>' : `${raw('**')}토요일 09:10${raw('**')}`} 서울역 집합${c(1)}</div>`,
    (link > 0 ? `<div class="ep url" style="opacity:${link}">https://example.com/stay</div>` : '') + (pop > 0 ? `<div class="lnk" data-a="rlink" style="opacity:${pop};transform:translateY(${(1 - pop) * 10}px)"><div class="lh"><i class="lfav">E</i><div><b>example.com</b><span>/stay</span></div></div><div class="lb"><i class="go">열기</i><i>주소 복사</i><i class="sub">주소 수정</i></div></div>` : ''),
    img > 0 ? `<div class="pic" data-a="rimg" style="opacity:${img};transform:scale(${.92 + .08 * img})"><img src="assets/sea.png"><i class="pb">⤢</i></div>` : '',
  ].join('');
  return win({ id: 'rich', ...RICH, title: '메모', bg: NOTE.yellow[0], right: `${icon('compass')}${icon('palette')}${icon('more')}`, body: `<div class="ed">${rows}</div><div class="save">저장됨</div>`, op, k, z });
}
