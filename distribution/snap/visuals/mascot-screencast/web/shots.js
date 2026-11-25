// 스토어(Microsoft Store·Snap Store) 스크린샷 4장: 영상과 같은 UI·마스코트로 한 장씩 그린다. render(n), n = 1..4
import * as UI from './ui.js';
import * as S from './sprite.js';

// 왼쪽 문구 + 오른쪽 실제 화면(앱 창은 월드 좌표 그대로, 확대·이동해 배치). at: 잎새가 딛는 앵커
const LINKS = [{ a: 'stay', b: 'photo', k: 1 }, { a: 'trip', b: 'train', k: 1 }, { a: 'trip', b: 'stay', k: 1 }];
// 영상 흐름과 같은 순서(찾기 → 묶기 → 잇기·진행 → 쓰기). 문구는 영상 자막과 같은 토스체
const SHOTS = [
  { no: '01 찾기', h: '어디 적었는지<br>몰라도 <em>검색</em> 한 번', s: '제목과 할 일까지 찾아서<br>한 목록에 보여 줘요.',
    app: () => UI.library({ typed: '숙소', query: '숙소', filter: 1, focus: true, caret: true, expand: 1 }), box: UI.LIB, k: .95, at: ['lib', .82], pose: { eyes: 'd', mouth: 'smile' } },
  { no: '02 묶기', h: '관련 메모는<br><em>프로젝트</em> 한 장에', s: '올리면 안에 든 메모가 펼쳐지고<br>두 번 누르면 안으로 들어가요.',
    app: () => UI.canvas({ inside: 0, fan: 1, done: 2 }), box: { x: 600, y: 230, w: 960, h: 560 }, k: 1.1, cx: 1300, clip: true, at: ['root-trip', .86], pose: { eyes: 'happy', mouth: 'smile', arms: 'wave' } },
  { no: '03 잇기·진행', h: '흩어진 메모를<br>하나의 <em>계획</em>으로', s: '메모끼리 잇고, 할 일을 체크하면<br>진행률이 바로 채워져요.',
    app: () => UI.canvas({ links: LINKS, done: 3, complete: true, hot: 'trip' }), box: UI.CV, k: .64, cx: 1290, at: ['card-trip', .78], pose: { eyes: 'happy', arms: 'up', mouth: 'smile' }, spark: true },
  { no: '04 쓰기', h: '쓰는 대로<br><em>서식</em>이 입혀져요', s: '제목·굵게·체크 목록부터<br>링크와 사진까지 메모 안에 담아요.',
    app: () => UI.rich({ head: true, bold: true, half: true, link: 1, img: 1, caret: -1 }), box: UI.RICH, k: .9, cx: 1300, at: ['rich', .86], pose: { eyes: 'd', mouth: 'smile' } },
];
// clip: 창 일부를 확대할 때 오른쪽 패널 안으로 잘라 문구와 겹치지 않게 한다
const CLIP = { x: 830, y: 110, w: 1030, h: 860 };
window.DUR = SHOTS.length + 1;
window.render = n => {
  const i = Math.max(1, Math.min(SHOTS.length, Math.round(n))) - 1, s = SHOTS[i], b = s.box;
  // 창 오른쪽 영역(x 820~1860)의 가운데에 맞춘다
  const k = s.k, ox = (s.cx || 1340) - (b.x + b.w / 2) * k, oy = 540 - (b.y + b.h / 2) * k;
  document.body.innerHTML = `<div class="shot"><div class="wall" style="inset:0"></div>
    ${s.clip ? `<div class="sh-clip" style="left:${CLIP.x}px;top:${CLIP.y}px;width:${CLIP.w}px;height:${CLIP.h}px"><div style="position:absolute;left:${-CLIP.x}px;top:${-CLIP.y}px">` : ''}<div class="sh-app" style="transform:translate(${ox}px,${oy}px) scale(${k})"><div id="desk">${s.app()}</div><div id="spr"></div></div>${s.clip ? '</div></div>' : ''}
    <div class="sh-t"><div class="no">${s.no}</div><h1>${s.h}</h1><p>${s.s}</p></div></div>`;
  const desk = document.getElementById('desk'), spr = document.getElementById('spr');
  const A = (key, fx) => { const r = desk.querySelector(`[data-a="${key}"]`).getBoundingClientRect(); return { x: (r.left + r.width * fx - ox) / k, y: (r.top - oy) / k }; };
  const p = s.at[1] === 'right' ? { x: UI.ED.x + UI.ED.w - 70, y: A(s.at[0], 0).y } : A(...s.at);
  let h = '';
  h += S.sprite({ ...s.pose, ...p, px: 9 / Math.sqrt(k), shadow: 1 });
  if (s.spark) for (let j = 0; j < 8; j++) { const a = j / 8 * Math.PI * 2; h += S.SPARK(p.x + Math.cos(a) * 120, p.y - 60 + Math.sin(a) * 80, 6, 1, j % 2 ? '#1f9d5c' : '#f2c94c'); }
  spr.innerHTML = h;
};
await document.fonts.load('20px "Noto Sans CJK KR"'); await document.fonts.load('bold 20px "Noto Sans CJK KR"');
window.render(1);
window.READY = true;
