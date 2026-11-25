// 타임라인(단일 기준): 화면 상태, 카메라, 잎새의 연기, 커서, HUD, 음향 이벤트. 이야기는 editorial/TREATMENT.md.
import { clamp, seg, eio, eo, lerp, hash } from '/core/lib.js';
import * as UI from './ui.js';
import * as S from './sprite.js';
import { around, spanAt, stagger } from './track.js';

export const W = 1920, H = 1080, DUR = 55, BEAT = .5;
export const EV = [];
const ev = (t, type, o = {}) => EV.push({ t: +t.toFixed(3), type, ...o });

// ── 비트 ── (TREATMENT.md 큐 맵)
export const T = {
  // 1막 혼돈
  slap: { s12: .35, s13: .85, s14: 1.35, s15: 2.6 }, peek: [1.75, 2.15], buried: 2.6, popOut: [3.05, 3.45],
  drag1: [3.7, 3.85, 4.4], drag2: [4.6, 4.75, 5.3], miss: [5.75, 6.0], toS12: [5.6, 6.0], rally: 6.3, hush: 6.35,
  wipe: [6.6, 7.6], cut: 7.1,
  // 2막 모으고 찾기
  toLib: [7.3, 7.75], gather: [7.9, .32, .28], toSearch: [10.4, 10.8], searchTap: 11.0, q1: 11.35, q2: 11.65, found: 11.8,
  // 3막 묶고 잇기: 전체 캔버스의 프로젝트 = 종이 더미 → 올리면 부채 → 두 번 눌러 서브그래프로(PROJECT-001)
  cvTap: 14.6, cvOpen: [14.75, 15.2], toRoot: [15.2, 15.8], fan: [16.1, 16.5], enter: 17.7, dive: [17.95, 18.7], toCard: [18.7, 19.3],
  d1: [19.8, 20.2, 21.1, 21.2], back: [21.5, 21.9], d2: [22.0, 22.2, 23.1, 23.2], toCb: [23.8, 24.2], done: 25.8,
  // 4막 쓰기: 한 동작에 1.4~2.2초. 커서가 닿은 줄만 서식 기호가 보인다(FMT-001), 진행 중 체크, 링크 미리보기, 사진(FMT-003·004)
  open: [28.4, 28.9], head: 30.0, bold: 31.4, half: 32.8, link: 34.4, img: 36.6, close: [38.6, 39.0],
  // 보상: 경로의 '전체'로 나오면 프로젝트가 다시 한 장의 더미로 접혀 있다(완료 3/3), 첫 장면과 같은 구도로 빠진다
  home: [39.5, 39.8, 40.6], toProj: [40.7, 41.3], fanEnd: [41.5, 41.9], pull: [43.4, 44.8],
  out: [46.4, 47.4], endIn: [47.7, 48.5], tag: 48.8, fade: [54.3, 55],
};
// 포스터 시점. build.sh가 events.json의 'still' 이벤트로 읽는다.
// 흩어졌던 메모가 한 장씩 목록으로 모인다(01 모으기). 화면과 pop 소리가 같은 정의를 쓴다.
const GATHER = stagger(T.gather[0], UI.MEMOS.length, T.gather[1], T.gather[2]);
const STILLS = { poster: 42.4 };
// 문제 → 약속 → 증거 → 보상
const CAPS = [
  [.3, 3.4, '메모가 늘수록 화면이 *가려져요*'],
  [3.6, 6.5, '정작 필요한 메모는 *어디 있을까요?*'],
  [7.7, 10.8, 'MemoIvy에선 메모가 *한 곳에* 모여요'],
  [11.0, 14.5, '어디 적었는지 몰라도 *검색*하면 바로 나와요'],
  [15.4, 18.6, '관련 메모는 *프로젝트* 한 장에 겹쳐 담겨요'],
  [18.9, 22.0, '프로젝트 안에서 메모를 *이으면*'],
  [22.2, 25.5, '*하나의 계획*이 돼요'],
  [25.8, 28.3, '할 일을 체크하면 *진행률*이 바로 채워져요'],
  [28.9, 33.9, '쓰는 대로 *서식*이 입혀지고'],
  [34.2, 38.8, '*링크*도 *사진*도 메모 안에 담아요'],
  [40.6, 43.3, '밖에서 보면 *한 장*으로 정리돼 있어요'],
  [43.6, 46.2, '화면도 다시 *깔끔해져요*'],
];
export const SUBS = CAPS.map(([t0, t1, text]) => ({ t0, t1, text: text.replace(/\*/g, '') }));
const CHAPS = [[7.8, 10.4, '01', '모으기'], [15.3, 17.6, '02', '묶고 잇기'], [25.8, 28.0, '03', '진행'], [28.9, 30.9, '04', '쓰기']];

// ── 카메라: [t, cx, cy, z], 키 사이 eio. 모니터 너머는 보이지 않게 clamp ──
const CAM = [
  [0, 960, 540, 1.0], [1.6, 960, 540, 1.0], [2.3, 1000, 470, 1.18], [4.5, 1060, 450, 1.18], [5.6, 1170, 380, 1.3], [7.0, 1170, 380, 1.3],
  [7.05, 960, 540, 1.0], [7.6, 960, 540, 1.0], [8.6, 960, 480, 1.25], [9.9, 960, 480, 1.25], [10.7, 960, 360, 1.6], [14.3, 960, 360, 1.6],
  [14.9, 960, 540, 1.0], [15.6, 960, 540, 1.0], [16.3, 920, 470, 1.3], [17.8, 920, 470, 1.3], [18.7, 960, 540, 1.0], [19.0, 960, 540, 1.0],
  [19.8, 700, 450, 1.45], [23.6, 700, 470, 1.45], [24.4, 470, 470, 1.95], [26.8, 470, 470, 1.95], [27.8, 960, 540, 1.0], [28.4, 960, 540, 1.0],
  [29.1, 1360, 600, 1.3], [38.6, 1360, 600, 1.3], [39.2, 960, 540, 1.0], [40.7, 960, 540, 1.0], [41.4, 920, 470, 1.3], [43.2, 920, 470, 1.3], [44.8, 960, 540, 1.0],
];
const clampCam = ([cx, cy, z]) => [clamp(cx, W / 2 / z, W - W / 2 / z), clamp(cy, H / 2 / z, H - H / 2 / z), z];
export function camAt(t) {
  const [a, b] = around(CAM, t); if (a === b) return clampCam(a.slice(1));
  const k = eio(seg(t, a[0], b[0]));
  return clampCam([lerp(a[1], b[1], k), lerp(a[2], b[2], k), Math.exp(lerp(Math.log(a[3]), Math.log(b[3]), k))]);
}
// 1막 철썩 순간만 화면이 떨린다(혼돈). 앱 동작 구간은 카메라가 흔들리지 않는다.
const PUNCH = Object.values(T.slap).map(s => [s, .025]);
const punch = t => PUNCH.reduce((z, [at, a]) => z + (t > at ? a * Math.exp(-(t - at) * 7) * Math.sin((t - at) * 26) : 0), 0);
export const blurAt = () => 0;

// ── 화면 상태 ──
const appear = (t, [a, b]) => { const k = eo(seg(t, a, b)); return { op: k, k: .94 + .06 * k }; };
const OFF = { s13: [T.drag1, [-250, 240]], s15: [T.drag2, [400, 150]] };
const offAt = t => Object.fromEntries(Object.entries(OFF).map(([id, [D, [dx, dy]]]) => { const k = eio(seg(t, D[1], D[2])); return [id, [dx * k, dy * k]]; }));
// 서브그래프 깊이(0 전체 ↔ 1 프로젝트 안)와 부채 펼침. 클릭하면 부채는 접힌다(실제 앱 .fan-quiet).
const insideAt = t => eio(seg(t, ...T.dive)) * (1 - eio(seg(t, T.home[1], T.home[2])));
const fanAt = t => eo(seg(t, ...T.fan)) * (1 - seg(t, T.enter - .05, T.enter + .15)) + eo(seg(t, ...T.fanEnd)) * (1 - eo(seg(t, T.pull[0] - .4, T.pull[0])));   // 끝에서도 커서를 올린 동안만 펼친다
// 검색: 입력한 글자 수만큼 보이고, 첫 글자에서 결과가 접히며 걸러진다(같은 결과 집합)
const SEARCH = { query: '숙소', typed: t => '숙소'.slice(0, (t >= T.q1) + (t >= T.q2)) };
function screens(t) {
  if (t < T.cut) return UI.stickies(t, { at: T.slap, off: offAt(t) });
  let h = '';
  if (t < T.cvOpen[1] + .1) {
    h += UI.library({
      enter: t < GATHER.end ? UI.MEMOS.map((_, i) => GATHER.k(t, i)) : null,
      typed: SEARCH.typed(t), query: SEARCH.query, filter: eio(seg(t, T.q1, T.q1 + .35)), expand: eo(seg(t, T.found, T.found + .35)),
      focus: t >= T.searchTap, caret: t >= T.searchTap,
    });
  }
  if (t >= T.cvOpen[0]) {
    const lk = (a, b, d) => ({ a, b, k: d ? eio(seg(t, d[3], d[3] + .25)) : 1 });
    const links = [lk('stay', 'photo')];
    if (t >= T.d1[3]) links.push(lk('trip', 'train', T.d1));
    if (t >= T.d2[3]) links.push(lk('trip', 'stay', T.d2));
    const dragOf = (D, to) => {
      if (t < D[0] || t >= D[3]) return null;
      const [p0, , , p3] = UI.linkPath('trip', to), k = eio(seg(t, D[1], D[2]));
      return { from: p0, to: [lerp(p0[0] + 10, p3[0], k), lerp(p0[1], p3[1], k) + Math.sin(k * Math.PI) * -30] };
    };
    const a = appear(t, T.cvOpen);
    h += UI.canvas({ inside: insideAt(t), fan: fanAt(t), op: a.op, k: a.k, links, drag: dragOf(T.d1, 'train') || dragOf(T.d2, 'stay'), done: t < T.done ? 2 : 3, complete: t >= T.done + .1, hot: t > T.done && t < T.done + .6 ? 'trip' : t > T.toProj[1] && t < T.toProj[1] + .8 ? 'root' : null });
  }
  if (t >= T.open[0] && t < T.close[1]) {
    const caret = t < T.head ? 0 : t < T.bold ? 1 : -1, a = appear(t, T.open), c = 1 - eio(seg(t, ...T.close));
    h += UI.rich({ head: caret !== 0, bold: caret !== 1, caret, half: t >= T.half, link: eo(seg(t, T.link, T.link + .2)), pop: eo(seg(t, T.link + .25, T.link + .5)) * (1 - seg(t, T.img - .35, T.img - .1)), img: eo(seg(t, T.img, T.img + .35)), op: a.op * c, k: a.k * (.96 + .04 * c) });
  }
  return h;
}
// 끌고 있는 연결선 끝(월드 좌표). 놓은 뒤에는 도착점에 머문다.
const dragTip = t => {
  if (t < T.d1[0]) return null;
  const [D, to] = t >= T.d2[0] ? [T.d2, 'stay'] : [T.d1, 'train'];
  const P = UI.linkPath('trip', to), k = eio(seg(t, D[1], D[2]));
  return UI.cvWorld([lerp(P[0][0] + 10, P[3][0], k), lerp(P[0][1], P[3][1], k) + Math.sin(k * Math.PI) * -30]);
};

// ── 잎새 동작 원시 ──
function jumpPose(t, t0, t1, p0, p1, h) {
  const k = seg(t, t0, t1), e = eio(k);
  return { x: lerp(p0.x, p1.x, e), y: lerp(p0.y, p1.y, e) - h * Math.sin(Math.PI * k), sq: -.18 * Math.sin(Math.PI * Math.min(1, k * 2.2)), legs: 'tuck', flip: p1.x < p0.x };
}
const crouch = (t, t0) => (t > t0 - .14 && t < t0 ? .26 * Math.sin(Math.PI * seg(t, t0 - .14, t0) * .5) : 0);
const landSq = (t, t1) => (t >= t1 && t < t1 + .45 ? .3 * Math.exp(-(t - t1) * 9) * Math.cos((t - t1) * 24) : 0);
const blink = (t, base) => (hash(Math.floor(t * 2.3) * 1.7) > .82 && (t * 2.3) % 1 < .18 ? 'blink' : base);
const breathe = t => .035 * Math.max(0, Math.sin((t / BEAT) * Math.PI * 2));
const walk = t => (Math.floor(t * 10) % 2 ? 'walkA' : 'walkB');

// 정거장(root는 오른쪽 끝: 부채 제목을 가리지 않는다): 실제 DOM 윗면(앵커) 또는 고정 월드 좌표
const ST = {
  below: () => ({ x: 1150, y: 1180 }), wipeP: () => ({ x: 1300, y: 300 }), hidden: () => null,
  s14: A => A('st-s14', .62, 0), s15: A => A('st-s15', .55, 0), s12: A => A('st-s12', .45, 0),
  libT: A => A('lib-title', .5, 0), search: A => A('search', .84, 0),
  card: A => A('card-trip', .78, 0), tip: (A, t) => { const p = dragTip(t); return p && { x: p[0], y: p[1] }; },
  stay: A => A('card-stay', .5, 0), photo: A => A('card-photo', .3, 0), root: A => A('root-trip', .86, 0), rich: A => A('rich-title', .5, 0),
};
// [시작, 끝, 정거장] 또는 [시작, 끝, 출발, 도착, 높이] (점프)
const PATH = [
  [...T.peek, 'below', 's14', 140], [T.peek[1], T.buried, 's14'], [T.buried, T.popOut[0], 'hidden'], [...T.popOut, 's14', 's15', 110], [T.popOut[1], T.toS12[0], 's15'],
  [...T.toS12, 's15', 's12', 120], [T.toS12[1], T.cut, 's12'], [T.cut, T.toLib[0], 'hidden'], [...T.toLib, 'wipeP', 'libT', 160], [T.toLib[1], T.toSearch[0], 'libT'],
  [...T.toSearch, 'libT', 'search', 80], [T.toSearch[1], T.toCard[0], 'search'], [...T.toRoot, 'search', 'root', 260], [T.toRoot[1], T.toCard[0], 'root'], [...T.toCard, 'root', 'card', 200], [T.toCard[1], T.d1[0], 'card'],
  [T.d1[0], T.d1[1], 'card', 'tip', 50], [T.d1[1], T.back[0], 'tip'], [...T.back, 'tip', 'card', 120], [T.back[1], T.d2[0], 'card'],
  [T.d2[0], T.d2[1], 'card', 'tip', 50], [T.d2[1], T.toCb[0], 'tip'], [...T.toCb, 'tip', 'card', 120], [T.toCb[1], T.open[1], 'card'], [T.open[1], T.open[1] + .5, 'card', 'rich', 220], [T.open[1] + .5, T.close[0], 'rich'], [T.close[0], T.close[0] + .5, 'rich', 'card', 220], [T.close[0] + .5, T.toProj[0], 'card'],
  [...T.toProj, 'card', 'root', 200], [T.toProj[1], T.out[1], 'root'],
];
export const JUMPS = PATH.filter(p => p.length === 5).map(p => [p[0], p[1]]);

// 구간별 연기(포즈·소품). 위치는 PATH가 정한다.
function act(t, c, fx) {
  c.px = t < T.cut ? 7 : 5;
  const at = (a, d = .8) => t > a && t < a + d;
  if (t < T.buried) { c.eyes = t > T.buried - .25 ? 'wide' : Math.floor(t * 3) % 2 ? 'l' : 'r'; if (t > T.buried - .25) c.mouth = 'o'; }
  else if (t < T.popOut[1]) Object.assign(c, { eyes: 'wide', mouth: 'o', arms: 'up' });
  else if (t < T.drag2[0]) { c.eyes = t < T.drag1[0] ? 'shut' : 'd'; if (t < T.drag1[0]) fx.push(S.SWEAT(c.x + 45, c.y - 85 - 20 * ((t * 2) % 1), 4, 1)); if (t > T.drag1[1]) fx.push(S.QUEST(c.x + 50, c.y - 100, 4, 1)); }
  else if (t < T.toS12[0]) Object.assign(c, t < T.drag2[2] ? { eyes: 'wide', mouth: 'o', arms: 'up' } : { eyes: 'shut', mouth: 'n' });
  else if (t < T.rally) { c.eyes = 'd'; c.arms = 'wave'; if (t > T.toS12[1]) fx.push(S.BANG(c.x + 45, c.y - 110, 4, 1)); }
  else if (t < T.cut) Object.assign(c, { eyes: 'happy', mouth: 'smile', arms: 'up' });
  else if (t < T.toSearch[0]) { c.eyes = t < T.toLib[1] + .8 ? 'happy' : 'd'; c.mouth = 'smile'; }
  else if (t < T.toRoot[0]) {
    c.eyes = t < T.q1 ? 'd' : 'l';
    if (t > T.found) { Object.assign(c, { eyes: 'happy', mouth: 'smile', arms: t < T.found + 1 ? 'up' : 'down' }); const k = seg(t, T.found, T.found + .7); if (k < 1) fx.push(S.SPARK(c.x - 50, c.y - 70 - 30 * k, 4, 1 - k), S.SPARK(c.x + 50, c.y - 90 - 40 * k, 3, 1 - k, '#5fa3ee')); }
  } else if (t < T.d1[1]) c.eyes = 'r';
  else if (t < T.toCb[1]) {
    c.eyes = 'r'; c.arms = 'up';
    for (const D of [T.d1, T.d2]) if (at(D[3])) { Object.assign(c, { eyes: 'happy', mouth: 'smile', arms: 'wave' }); const k = seg(t, D[3], D[3] + .6); fx.push(S.SPARK(c.x - 40, c.y - 80 - 30 * k, 4, 1 - k), S.SPARK(c.x + 40, c.y - 90 - 40 * k, 3, 1 - k, '#5fa3ee')); }
    if (t > T.back[0] && t < T.d2[1]) c.arms = 'down';
  } else if (t < T.toProj[0]) {
    c.eyes = t < T.done ? 'd' : 'happy';
    if (t >= T.done) {
      Object.assign(c, { mouth: 'smile', arms: t < T.done + 1.2 ? 'up' : 'down' });
      if (t < T.done + .35) Object.assign(c, jumpPose(t, T.done, T.done + .35, c, c, 40));
      const k = seg(t, T.done, T.done + .8); if (k < 1) for (let i = 0; i < 8; i++) { const a = i / 8 * Math.PI * 2; fx.push(S.SPARK(c.x + Math.cos(a) * (40 + k * 90), c.y - 50 + Math.sin(a) * (30 + k * 60), 4, 1 - k, i % 2 ? '#1f9d5c' : '#f2c94c')); }
    }
  } else if (t < T.pull[1]) {   // 완료된 프로젝트(2/2) 위에서 박자에 맞춰 손 흔들기
    Object.assign(c, { eyes: 'happy', mouth: 'smile' }); if (t > T.toProj[1]) c.arms = Math.floor(t / BEAT) % 2 ? 'wave' : 'up';
    const k = seg(t, T.toProj[1], T.toProj[1] + .7); if (k > 0 && k < 1) fx.push(S.SPARK(c.x - 50, c.y - 70 - 30 * k, 4, 1 - k), S.SPARK(c.x + 50, c.y - 90 - 40 * k, 3, 1 - k, '#1f9d5c'));
  }
  else {   // 맑아진 화면에서 햇볕 쬐기(1막 깔림의 보상)
    Object.assign(c, { eyes: 'shut', mouth: 'smile', arms: 'up', sq: .12 + breathe(t) * 1.5 });
    fx.push(S.SUN(c.x + 90, c.y - 120, 4, .5 + .5 * Math.sin(t * 3)));
  }
}

function mascot(t, A) {
  if (t < T.peek[0] || t >= T.out[1]) return null;
  const P = spanAt(PATH, t); if (!P) return null;
  let c;
  if (P.length === 5) {
    const a = ST[P[2]](A, P[0]), b = ST[P[3]](A, t); if (!a || !b) return null;
    c = jumpPose(t, P[0], P[1], a, b, P[4]);
  } else { const p = ST[P[2]](A, t); if (!p) return null; c = { legs: 'stand', ...p }; }
  const fx = []; act(t, c, fx);
  if (P[2] === 'tip' && P.length === 3) c.legs = t < T.d1[2] || (t > T.d2[1] && t < T.d2[2]) ? walk(t) : 'stand';
  for (const [j] of JUMPS) c.sq = (c.sq || 0) + crouch(t, j);
  for (const [, l] of JUMPS) c.sq = (c.sq || 0) + landSq(t, l);
  if (!c.eyes || c.eyes === 'n') c.eyes = blink(t, 'n');
  if (c.legs !== 'tuck') c.sq = (c.sq || 0) + breathe(t) * .6;
  return { c, fx };
}

// ── 커서(사용자) ── {t, k: 앵커, fx, fy} 또는 {t, drag}. click: 누름, down/up: 끌기
const CUR = [
  { t: 3.4, k: 'st-s13', fx: 1.2, fy: 1.2, op: 0 }, { t: T.drag1[0], k: 'st-s13', fx: .5, fy: .25 }, { t: T.drag1[0] + .01, down: 1, k: 'st-s13', fx: .5, fy: .25 }, { t: T.drag1[2], up: 1, k: 'st-s13', fx: .5, fy: .25 },
  { t: T.drag2[0], down: 1, k: 'st-s15', fx: .3, fy: .7 }, { t: T.drag2[2], up: 1, k: 'st-s15', fx: .3, fy: .7 },
  { t: T.miss[0], click: 1, k: 'st-s2', fx: .6, fy: .6 }, { t: T.miss[1], click: 1, k: 'st-s10', fx: .4, fy: .5 }, { t: 6.5, k: 'st-s10', fx: .4, fy: 1.5, op: 0 },
  { t: 10.6, k: 'search', fx: .3, fy: 2.5, op: 0 }, { t: 10.85, k: 'search', fx: .35, fy: .55 }, { t: T.searchTap, click: 1, k: 'search', fx: .35, fy: .55 }, { t: 11.4, k: 'search', fx: .4, fy: 1.6, op: 0 },
  { t: 14.0, k: 'canvasBtn', fx: .5, fy: 2, op: 0 }, { t: 14.45, k: 'canvasBtn', fx: .5, fy: .55 }, { t: T.cvTap, click: 1, k: 'canvasBtn', fx: .5, fy: .55 }, { t: 15.1, k: 'canvasBtn', fx: .5, fy: 3, op: 0 },
  { t: 15.5, k: 'root-trip', fx: .75, fy: 1.5, op: 0 }, { t: 16.0, k: 'root-trip', fx: .62, fy: .45 }, { t: T.enter, click: 1, k: 'root-trip', fx: .62, fy: .45 }, { t: T.enter + .2, click: 1, k: 'root-trip', fx: .62, fy: .45 }, { t: 18.3, k: 'root-trip', fx: .7, fy: 1.4, op: 0 },
  { t: 19.2, k: 'card-trip', fx: .9, fy: 1.2, op: 0 }, { t: 19.7, k: 'card-trip', fx: 1, fy: .5 }, { t: T.d1[0], down: 1, k: 'card-trip', fx: 1, fy: .5 }, { t: T.d1[1], drag: 1 }, { t: T.d1[2], drag: 1 }, { t: T.d1[3], up: 1, drag: 1 },
  { t: T.back[1], k: 'card-trip', fx: 1.1, fy: .7 }, { t: T.d2[0], down: 1, k: 'card-trip', fx: 1, fy: .5 }, { t: T.d2[1], drag: 1 }, { t: T.d2[2], drag: 1 }, { t: T.d2[3], up: 1, drag: 1 },
  { t: T.done - .7, k: 'cv-cb2', fx: 1.5, fy: 1.5 }, { t: T.done - .15, k: 'cv-cb2', fx: .5, fy: .6 }, { t: T.done, click: 1, k: 'cv-cb2', fx: .5, fy: .6 }, { t: T.done + .8, k: 'cv-cb2', fx: 2, fy: 2, op: 0 },
  { t: T.half - .8, k: 'rcb3', fx: 3, fy: 3, op: 0 }, { t: T.half - .25, k: 'rcb3', fx: .5, fy: .5 }, { t: T.half, click: 1, k: 'rcb3', fx: .5, fy: .5 }, { t: T.half + .7, k: 'rcb3', fx: 3, fy: 1.5, op: 0 },
  { t: T.img - .9, k: 'rich', fx: 1.15, fy: .82, op: 0 }, { t: T.img - .55, down: 1, k: 'rich', fx: 1.02, fy: .82 }, { t: T.img, up: 1, k: 'rich', fx: .55, fy: .8 }, { t: T.img + .7, k: 'rich', fx: .6, fy: 1.1, op: 0 },
  { t: T.home[0] - .6, k: 'crumb-home', fx: 2, fy: 3, op: 0 }, { t: T.home[0] - .15, k: 'crumb-home', fx: .5, fy: .6 }, { t: T.home[0], click: 1, k: 'crumb-home', fx: .5, fy: .6 }, { t: T.home[0] + .6, k: 'crumb-home', fx: 1, fy: 3, op: 0 },
  { t: T.fanEnd[0] - .6, k: 'root-trip', fx: .35, fy: 1.5, op: 0 }, { t: T.fanEnd[0] - .05, k: 'root-trip', fx: .35, fy: .6 }, { t: T.pull[0] - .4, k: 'root-trip', fx: .35, fy: .6 }, { t: T.pull[0] + .3, k: 'root-trip', fx: .4, fy: 1.6, op: 0 },
];
// 한 번만 정규화: 누름(click·down) 시각과 누른 채 끌기(down→up) 구간
const CURK = CUR.map(k => ({ ...k, op: k.op ?? 1 }));
const PRESS = CURK.filter(q => q.click || q.down).map(q => [q.t, q.t + .4, q]);
const HOLD = CURK.flatMap((q, j) => (q.down ? [[q.t, CURK.slice(j).find(u => u.up).t]] : []));
function cursor(t, A) {
  const keys = CURK;
  if (t < keys[0].t || t > keys[keys.length - 1].t) return null;
  const [a, b] = around(keys, t, q => q.t);
  const at = (q, tt) => { if (q.drag) { const p = dragTip(tt); return p && { x: p[0], y: p[1] + 4 }; } return A(q.k, q.fx, q.fy); };
  const pa = at(a, a.t), pb = at(b, b.t); if (!pa || !pb) return null;
  if (a.drag && b.drag) { const p = at(a, t); return { ...p, op: 1, down: true }; }
  const k = a === b ? 0 : eio(seg(t, a.t, b.t)), op = lerp(a.op, b.op, k);
  if (op <= .01) return null;
  const click = spanAt(PRESS, t)?.[2];
  const held = !!spanAt(HOLD, t);
  return { x: lerp(pa.x, pb.x, k), y: lerp(pa.y, pb.y, k), op, ring: click ? seg(t, click.t, click.t + .4) : 0, down: held || (click && t < click.t + .12) };
}

// ── HUD ──
const env = (t, a, b, fi, fo = fi) => Math.min(seg(t, a, a + fi), 1 - seg(t, b - fo, b));
function hud(t) {
  const Hd = {};
  const cap = spanAt(CAPS, t);
  if (cap) { const [a, b, text] = cap; Hd.cap = { text, op: env(t, a, b, .25), y: (1 - eo(seg(t, a, a + .3))) * 20 }; }
  const ch = spanAt(CHAPS, t); if (ch) Hd.chap = { no: ch[2], nm: ch[3], op: env(t, ch[0], ch[1], .3) };
  // 반전 와이프는 잎새가 선 자리(화면 좌표)에서 퍼진다
  const KEYS = [[T.half - .6, T.half + .8, ['Shift', '클릭'], T.half], [T.link - .5, T.link + .9, ['Ctrl', 'V'], T.link]];
  const kk = spanAt(KEYS, t); if (kk) Hd.keys = { keys: kk[2], op: env(t, kk[0], kk[1], .2), down: t > kk[3] - .08 && t < kk[3] + .15 };
  if (t > T.wipe[0] && t < T.wipe[1]) Hd.wipe = { p: seg(t, ...T.wipe), o: [1220, 330] };
  if (t > T.out[0] && t < T.out[1]) Hd.wipe = { p: seg(t, ...T.out), o: [960, 540] };
  if (t >= (T.out[0] + T.out[1]) / 2) {
    const land = T.endIn[1];
    Hd.end = { tag: eo(seg(t, T.tag, T.tag + .6)), sub: eo(seg(t, T.tag + .5, T.tag + 1.1)), cta: eo(seg(t, T.tag + 1.0, T.tag + 1.6)), fade: seg(t, ...T.fade), hop: t < T.endIn[0] ? null : { k: seg(t, ...T.endIn), h: 220, wave: t > land + .6 ? (Math.floor(t / BEAT) % 2 ? 'wave' : 'up') : 'up', after: t >= land, sq: landSq(t, land) } };
  }
  return Hd;
}

export function frame(t) {
  const cam = camAt(t);
  return { cam, z: cam[2] + punch(t), blur: blurAt(t), html: screens(t), mascot: A => mascot(t, A), cursor: A => cursor(t, A), hud: hud(t) };
}

// ── 음향 이벤트(mix.py가 읽는다) ──
function buildEvents() {
  for (const at of Object.values(T.slap)) ev(at, 'slap');
  ev(T.buried + .02, 'squish');
  for (const D of [T.drag1, T.drag2]) { ev(D[0], 'click'); ev(D[1], 'shuffle', { d: D[2] - D[1] }); }
  ev(T.drag2[1], 'whee');
  for (const at of T.miss) ev(at, 'miss');
  ev(T.toS12[1] + .05, 'bang'); ev(T.rally, 'rally'); ev(T.wipe[0], 'wipe'); ev(T.out[0], 'wipe');
  for (const at of [T.q1, T.q2]) { ev(at - .06, 'key'); ev(at - .02, 'key'); }
  for (const at of [T.searchTap, T.cvTap, T.enter, T.done, T.d1[0], T.d2[0]]) ev(at, 'click');
  ev(T.found, 'found'); ev(T.cvOpen[0], 'open'); ev(T.home[0], 'click'); ev(T.home[1], 'whoosh', { d: .5 }); ev(T.toProj[1] + .02, 'found'); ev(T.fan[0], 'whoosh', { d: .3 }); ev(T.enter + .2, 'click'); ev(T.dive[0], 'whoosh', { d: .6 }); ev(T.fanEnd[0], 'whoosh', { d: .3 });
  for (const at of [T.d1[3], T.d2[3]]) ev(at, 'snap');
  for (const D of [T.d1, T.d2]) ev(D[1], 'drag', { d: D[2] - D[1] });
  ev(T.done, 'done');
  ev(T.open[0], 'open'); ev(T.close[0], 'whoosh', { d: .35 });
  for (const at of [T.head, T.bold]) ev(at - .04, 'key');
  for (const at of GATHER.times) ev(at + .1, 'pop');
  ev(T.half, 'click'); ev(T.half + .02, 'check', { n: 1 }); ev(T.link - .05, 'key'); ev(T.link, 'pop'); ev(T.img, 'pop');
  for (const [a, b] of JUMPS) { ev(a, 'jump'); ev(b, 'land'); }
  ev(T.endIn[0], 'jump'); ev(T.endIn[1], 'land');
  for (let q = T.pull[1]; q < T.out[0]; q += 1.0) ev(q, 'bird');
  // 음악 구간
  ev(0, 'cue', { id: 'chaos' }); ev(T.hush, 'cue', { id: 'hush' }); ev(T.cut, 'cue', { id: 'reveal' }); ev(T.cvOpen[0], 'cue', { id: 'connect' });
  ev(T.done, 'cue', { id: 'done' }); ev(T.home[1], 'cue', { id: 'grow' }); ev(T.pull[0], 'cue', { id: 'clean' }); ev(T.out[0], 'cue', { id: 'pre' }); ev(T.endIn[1], 'cue', { id: 'end' });
}
buildEvents();
ev(0, 'tempo', { beat: BEAT });
for (const [name, at] of Object.entries(STILLS)) ev(at, 'still', { name });
for (const s of SUBS) ev(s.t0, 'sub', { t1: s.t1, text: s.text });
