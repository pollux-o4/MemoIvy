import { ACCENT, FONT } from './theme.mjs';

/** SVG 문자열 조립 기본 요소. 모든 그래픽이 같은 모서리·선·글자 규칙을 공유한다. */
const escape = (value) => String(value).replace(/[&<>"]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[ch]);

/** 글자 조각: 문자열이거나 { text, accent, mono, bold } 강조 조각. */
const span = (part, palette) => typeof part === 'string' ? escape(part) : `<tspan${part.accent ? ` fill="${ACCENT}" font-weight="700"` : ''}${part.mono ? ` font-family="${FONT.mono}" fill="${palette.fg}"` : ''}>${escape(part.text)}</tspan>`;

export function text(palette, { x, y, value, size = 19, color = palette.sub, weight = 400, mono = false, anchor = 'start', spacing }) {
  const parts = Array.isArray(value) ? value : [value];
  return `<text x="${x}" y="${y}" font-family="${mono ? FONT.mono : FONT.sans}" font-size="${size}" font-weight="${weight}" fill="${color}"${anchor === 'start' ? '' : ` text-anchor="${anchor}"`}${spacing ? ` letter-spacing="${spacing}"` : ''}>${parts.map((part) => span(part, palette)).join('')}</text>`;
}

export const panel = (palette, { width, height, radius = 14 }) =>
  `<rect x=".5" y=".5" width="${width - 1}" height="${height - 1}" rx="${radius}" fill="${palette.surface}" stroke="${palette.line}"/>`;

export const at = (x, y, body) => `<g transform="translate(${x},${y})">${body}</g>`;

/** 바깥 링크 표시(↗). 버튼·카드가 같은 모양을 쓴다. */
export const outArrow = (x, y, color = ACCENT, width = 1.6) =>
  `<path d="M${x},${y} l7,-7 M${x + 2},${y - 7} h5 v5" fill="none" stroke="${color}" stroke-width="${width}" stroke-linecap="round"/>`;

/** 고정폭 글꼴은 글자 폭이 일정해(약 0.6em) 칩 너비를 문구 길이로 계산한다. */
const MONO_ADVANCE = 0.6;
export function chip(palette, { x, y, value, size = 15, padding = 12 }) {
  const width = Math.ceil(value.length * size * MONO_ADVANCE + padding * 2);
  return `<rect x="${x}" y="${y - 21}" width="${width}" height="30" rx="6" fill="${palette.chip}" stroke="${palette.line}"/>${text(palette, { x: x + padding, y, value, size, color: palette.fg, weight: 500, mono: true })}`;
}

export const svg = ({ width, height }, body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${body}</svg>`;

/** 같은 크기 카드를 가로로 나눠 그린다. 카드 내용은 호출자가 정한다. */
export function cardRow(palette, { width, height, gap = 24, cards }) {
  const cardWidth = (width - gap * (cards.length - 1)) / cards.length;
  return cards.map((draw, index) => at(+(index * (cardWidth + gap)).toFixed(1), 0, panel(palette, { width: cardWidth, height }) + draw(cardWidth))).join('');
}
