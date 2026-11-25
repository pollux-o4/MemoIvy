// 잎새: 덩굴 모자를 쓴 메모 도트 마스코트. 14×12 격자(좌우 팔 칸 포함), 발 아래 중앙이 기준점.
export const INK = '#1f3d2a', LEAF = '#42845b', VEIN = '#a9c788', STICKY = '#ffdf70';

// 덩굴 모자를 쓴 메모(열 1..12). o 외곽선, # 잎, + 잎맥, y 메모, Y 접힌 귀퉁이.
const BODY = ['...oo..oo...', '.oo##oo##oo.', 'o##+####+##o', 'oyyyyyyyyyyo', 'oyyyyyyyyyyo', 'oyyyyyyyyyyo',
  'oyyyyyyyyyyo', 'oyyyyyyyyYYo', 'oyyyyyyyYYo.', 'ooooooooo...'];
const EYES = {
  n: [[3, 4], [3, 5], [8, 4], [8, 5]], l: [[2, 4], [2, 5], [7, 4], [7, 5]], r: [[4, 4], [4, 5], [9, 4], [9, 5]],
  d: [[3, 5], [3, 6], [8, 5], [8, 6]], u: [[3, 3], [3, 4], [8, 3], [8, 4]], wide: [[3, 3], [3, 4], [3, 5], [8, 3], [8, 4], [8, 5]],
  shut: [[2, 5], [3, 5], [4, 5], [7, 5], [8, 5], [9, 5]], blink: [[3, 5], [8, 5]], happy: [[2, 5], [3, 4], [4, 5], [7, 5], [8, 4], [9, 5]],
};
const MOUTH = { n: [[5, 7], [6, 7]], o: [[5, 7], [6, 7], [5, 8], [6, 8]], smile: [[4, 7], [5, 8], [6, 8], [7, 7]], none: [] };
const LEGS = { stand: [[3, 2], [8, 2]], walkA: [[3, 2], [8, 1]], walkB: [[3, 1], [8, 2]], tuck: [[3, 1], [8, 1]], none: [] };
const ARMS = { down: [[0, 5], [0, 6], [13, 5], [13, 6]], up: [[0, 2], [0, 3], [13, 2], [13, 3]], wave: [[0, 5], [0, 6], [13, 2], [13, 3]], none: [] };
const FILL = { 1: INK, 2: LEAF, 3: VEIN, 4: STICKY, 5: '#e8b93f' };

function grid({ eyes = 'n', mouth = 'n', arms = 'down', legs = 'stand' }) {
  const P = new Map(), put = (c, r, v) => P.set(c + ',' + r, v);
  BODY.forEach((row, r) => [...row].forEach((ch, c) => { if (ch !== '.') put(c + 1, r, { o: 1, '+': 3, y: 4, Y: 5 }[ch] || 2); }));
  for (const [c, r] of EYES[eyes]) put(c + 1, r, 1);
  for (const [c, r] of MOUTH[mouth]) put(c + 1, r, 1);
  for (const [c, r] of ARMS[arms]) put(c, r, 1);
  for (const [c, h] of LEGS[legs]) for (let r = 10; r < 10 + h; r++) put(c + 1, r, 1);
  return [...P].map(([k, v]) => { const [c, r] = k.split(',').map(Number); return [c, r, v]; });
}

// x,y: 발 아래 중앙(월드 좌표), px: 도트 한 변, sq>0 눌림 / <0 늘어남
export function sprite({ x, y, px = 10, flip = false, sq = 0, rot = 0, op = 1, shadow = 0, shake = 0, ...pose }) {
  let s = '';
  for (let [c, r, v] of grid(pose)) { if (flip) c = 13 - c; s += `<rect x="${c}" y="${r}" width="1.04" height="1.04" fill="${FILL[v]}"/>`; }
  const sy = 1 - sq, sx = 1 + sq * .7, w = 14 * px * sx, h = 12 * px * sy;
  const sh = shadow > 0 ? `<div class="ps" style="left:${x - 5 * px}px;top:${y - px * .35}px;width:${10 * px}px;height:${px * .7}px;opacity:${.16 * shadow}"></div>` : '';
  return sh + `<svg class="px" viewBox="0 0 14 12" width="${w}" height="${h}" style="left:${x - w / 2 + shake}px;top:${y - h}px;opacity:${op};transform:rotate(${rot}rad);transform-origin:50% 90%" shape-rendering="crispEdges">${s}</svg>`;
}

export function pixart(rows, { x, y, px = 6, pal, op = 1, rot = 0 }) {
  let s = ''; rows.forEach((row, r) => [...row].forEach((ch, c) => { if (pal[ch]) s += `<rect x="${c}" y="${r}" width="1.04" height="1.04" fill="${pal[ch]}"/>`; }));
  const W = rows[0].length, H = rows.length;
  return `<svg class="px" viewBox="0 0 ${W} ${H}" width="${W * px}" height="${H * px}" style="left:${x}px;top:${y}px;opacity:${op};transform:rotate(${rot}rad)" shape-rendering="crispEdges">${s}</svg>`;
}
const C = (rows, pal) => (x, y, px, op = 1) => pixart(rows, { x: x - rows[0].length * px / 2, y: y - rows.length * px / 2, px, op, pal });
export const SPARK = (x, y, px, op, col = '#f2c94c') => C(['..y..', '..y..', 'yy.yy', '..y..', '..y..'], { y: col })(x, y, px, op);
export const CHECK = C(['.....g', '....gg', 'g..gg.', 'gggg..', '.gg...'], { g: '#1f9d5c' });
export const QUEST = C(['.kkk.', 'k...k', '...k.', '..k..', '.....', '..k..'], { k: INK });
export const HEART = C(['.r.r.', 'rrrrr', 'rrrrr', '.rrr.', '..r..'], { r: '#ec86a2' });
export function dust(x, y, t, px, n = 6, col = '#c9d3cb') {
  if (t <= 0 || t >= 1) return ''; let s = '';
  for (let i = 0; i < n; i++) {
    const side = i % 2 ? 1 : -1, k = (i >> 1) + 1, e = 1 - Math.pow(1 - t, 2);
    const dx = side * (px * 1.6 + k * px * 1.4) * e, dy = -(px * .6 + k * px * .5) * Math.sin(Math.PI * t * .9), sz = px * .7 * (1 - t * .6);
    s += `<i class="dot" style="left:${x + dx - sz / 2}px;top:${y + dy - sz}px;width:${sz}px;height:${sz}px;background:${col};opacity:${1 - t}"></i>`;
  }
  return s;
}
export const SWEAT = C(['.b.', 'bbb', 'bbb', '.b.'], { b: '#5fa3ee' });
export const BANG = C(['kk', 'kk', 'kk', 'kk', '..', 'kk'], { k: '#e2483d' });
export const SUN = C(['..y..', 'y.y.y', '.yyy.', 'yyyyy', '.yyy.', 'y.y.y', '..y..'], { y: '#f2c94c' });
