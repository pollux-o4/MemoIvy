// 렌더러: film.frame(t) → DOM. 카메라 변환, 앵커 측정, 스프라이트·커서·HUD 그리기.
import { clamp, hash, seg, lerp, eio } from '/core/lib.js';
import * as FM from './film.js';
import * as S from './sprite.js';

const { W, H } = FM;
const world = document.getElementById('world'), blurw = document.getElementById('blurw'), desk = document.getElementById('desk'), spr = document.getElementById('spr'), hudEl = document.getElementById('hud');
const Q = new URLSearchParams(location.search), NOSUBS = Q.has('nosubs');

window.DUR = FM.DUR;
window.EV = FM.EV.slice().sort((a, b) => a.t - b.t);
window.SUBS = FM.SUBS;
// 읽어야 하는 화면 글자: 엔드 카드(자막은 SRT로 따로 검사)
// 엔드 카드 문구(SSoT): 배포처는 Microsoft Store와 Snap Store
const END = { tag: '작은 메모에서, 연결된 생각으로.', sub: 'Windows·Linux용 무료·오픈소스 메모 앱', url: 'github.com/pollux-o4/MemoIvy' };
window.TEXTS = t => (t >= FM.T.tag + 1.6 && t < FM.T.fade[0] ? [{ id: 'tag', text: END.tag, x0: 560, y0: 580, x1: 1360, y1: 640 }, { id: 'sub', text: END.sub, x0: 660, y0: 655, x1: 1260, y1: 700 }] : []);

let lastHtml = '';
function render(t) {
  const F = FM.frame(t), [cx, cy] = F.cam, z = F.z;
  world.style.transform = `translate(${W / 2}px,${H / 2}px) scale(${z}) translate(${-cx}px,${-cy}px)`;
  blurw.style.filter = F.blur > .3 ? `blur(${F.blur}px)` : '';
  if (F.html !== lastHtml) { desk.innerHTML = F.html; lastHtml = F.html; }
  const A = (key, fx = .5, fy = 0) => {
    const el = desk.querySelector(`[data-a="${key}"]`); if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: (r.left + r.width * fx - W / 2) / z + cx, y: (r.top + r.height * fy - H / 2) / z + cy, w: r.width / z, h: r.height / z };
  };
  let h = '';
  const m = F.mascot(A);
  if (m) {
    const c = m.c; c.x = Math.round(c.x / (c.px / 2)) * (c.px / 2); c.y = Math.round(c.y / (c.px / 2)) * (c.px / 2);   // 반 도트 단위 스냅
    h += S.sprite({ ...c, shadow: c.legs === 'tuck' ? 0 : 1 }) + m.fx.join('');
    for (const [, l] of FM.JUMPS) if (t >= l && t < l + .4) h += S.dust(c.x, c.y, seg(t, l, l + .4), Math.max(4, c.px * .6));
  }
  const cu = F.cursor(A);
  if (cu) {
    if (cu.ring) { const r = 14 + 26 * cu.ring; h += `<div class="cring" style="left:${cu.x - r}px;top:${cu.y - r}px;width:${2 * r}px;height:${2 * r}px;opacity:${(1 - cu.ring) * .8}"></div>`; }
    h += `<svg class="cur" viewBox="0 0 24 24" width="30" height="30" style="left:${cu.x - 5}px;top:${cu.y - 3}px;opacity:${cu.op};transform:scale(${cu.down ? .88 : 1})"><path d="M5 3l14 8.5-6.2 1.3 3.6 6.8-2.6 1.3-3.5-6.9L5 18.8z" fill="#1d1d1f" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
  }
  spr.innerHTML = h;
  hud(F.hud, t);
}

const ICON = await (await fetch('assets/memoivy.svg')).text();
function hud(Hd, t) {
  let h = '';
  if (Hd.chap) h += `<div class="chap" style="opacity:${Hd.chap.op}"><b>${Hd.chap.no}</b>${Hd.chap.nm}</div>`;
  if (Hd.keys) h += `<div class="keys" style="opacity:${Hd.keys.op}">${Hd.keys.keys.map(k => `<kbd class="${Hd.keys.down ? 'dn' : ''}">${k}</kbd>`).join('<span>+</span>')}</div>`;
  if (Hd.cap && !NOSUBS) h += `<div class="cap" style="opacity:${Hd.cap.op};transform:translate(-50%,${Hd.cap.y}px)">${Hd.cap.text.replace(/\*([^*]+)\*/g, '<em>$1</em>')}</div>`;
  if (Hd.end) h += endCard(Hd.end);
  if (Hd.wipe) h += wipe(Hd.wipe);
  if (t < .3) h += `<div class="fade" style="opacity:${1 - t / .3}"></div>`;
  hudEl.innerHTML = h;
  if (Hd.end && Hd.end.hop) {   // 잎새는 워드마크 글자 윗면에 착지(실측)
    const r = hudEl.querySelector('.wmk').getBoundingClientRect(), to = { x: r.left + r.width * .86, y: r.top + 16 }, from = { x: 2000, y: 760 };
    const { k, h: hh, wave, after, sq } = Hd.end.hop, e = eio(k);
    const p = after ? to : { x: lerp(from.x, to.x, e), y: lerp(from.y, to.y, e) - hh * Math.sin(Math.PI * k) };
    hudEl.insertAdjacentHTML('beforeend', S.sprite({ ...p, px: 10, flip: !after, legs: after ? 'stand' : 'tuck', eyes: after ? 'happy' : 'l', mouth: after ? 'smile' : 'n', arms: after ? wave : 'down', sq, shadow: after ? 1 : 0 }));
    if (Hd.end.fade) hudEl.insertAdjacentHTML('beforeend', `<div class="fade" style="opacity:${Hd.end.fade}"></div>`);
  }
}
const WIN = '<svg viewBox="0 0 20 20" width="26" height="26"><path fill="currentColor" d="M1 3l7.5-1v7.3H1zM9.5 1.8L19 .5v8.8H9.5zM1 10.4h7.5v7.3L1 16.7zM9.5 10.4H19v8.9l-9.5-1.3z"/></svg>';
// 리눅스 펭귄(단순화한 실루엣): 몸통·배·눈·부리·발
const TUX = '<svg viewBox="0 0 20 22" width="24" height="26"><ellipse cx="10" cy="12" rx="7" ry="9" fill="#111"/><ellipse cx="10" cy="14.5" rx="4.6" ry="6" fill="#f4f4f4"/><circle cx="8" cy="6.5" r="1.4" fill="#fff"/><circle cx="12" cy="6.5" r="1.4" fill="#fff"/><circle cx="8.3" cy="6.7" r=".6" fill="#111"/><circle cx="11.7" cy="6.7" r=".6" fill="#111"/><path d="M7.6 8.6h4.8L10 10.6z" fill="#f2b632"/><ellipse cx="6.4" cy="20.6" rx="2.8" ry="1.2" fill="#f2b632"/><ellipse cx="13.6" cy="20.6" rx="2.8" ry="1.2" fill="#f2b632"/></svg>';
function endCard(E) {
  return `<div class="end"><div class="eicon">${ICON}</div><div class="wmk">MemoIvy</div>
    <div class="etag" style="opacity:${E.tag};transform:translateY(${(1 - E.tag) * 16}px)">${END.tag}</div>
    <div class="esub" style="opacity:${E.sub}">${END.sub}</div>
    <div class="ecta" style="opacity:${E.cta};transform:translateY(${(1 - E.cta) * 12}px)"><span class="chip">${WIN}Microsoft Store</span><span class="chip">${TUX}<code>snap install memoivy</code></span></div>
    <div class="eurl" style="opacity:${E.cta}">${END.url}</div>${E.hop ? '' : E.fade ? `<div class="fade" style="opacity:${E.fade}"></div>` : ''}</div>`;
}
// 도트 와이프: 아이비 색 블록이 원점(잎새 자리)에서 퍼져 덮였다 걷힌다
function wipe({ p, o }) {
  const Sz = 120, cols = 16, rows = 9, pal = ['#33734c', '#42845b', '#2a5e3e'];
  let s = '<div class="wipe">';
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
    const d = Math.hypot((c + .5) * Sz - o[0], (r + .5) * Sz - o[1]) / 2000 * .7 + hash(c * 13.1 + r * 7.7) * .12;
    const a = clamp((p / .42 - d) / .18), b = clamp(((p - .55) / .42 - d) / .18), k = a * (1 - b);
    if (k <= 0) continue; const sz = Sz * k;
    s += `<b style="left:${c * Sz + (Sz - sz) / 2}px;top:${r * Sz + (Sz - sz) / 2}px;width:${sz + .5}px;height:${sz + .5}px;background:${pal[Math.floor(hash(c * 3.7 + r * 11.3) * 3)]}"></b>`;
  }
  return s + '</div>';
}

window.render = render;
await document.fonts.load('20px "Noto Sans CJK KR"'); await document.fonts.load('bold 20px "Noto Sans CJK KR"');
await Promise.all(['assets/memoivy.svg', 'assets/memoivy-noleaf.svg'].map(src => new Promise(r => { const i = new Image(); i.onload = i.onerror = r; i.src = src; })));
render(+(Q.get('t') || 0));
window.READY = true;
