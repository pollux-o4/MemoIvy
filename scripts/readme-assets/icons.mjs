import { ACCENT } from './theme.mjs';

/** 아이콘 레지스트리: 이름 → (palette) => 40px 내외 SVG 조각. 새 아이콘은 여기에만 더한다. */
const stroked = (palette, width, body) =>
  `<g fill="none" stroke="${palette.fg}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">${body}</g>`;

export const ICONS = {
  windows: () => `<rect width="8" height="8" fill="#f25022"/><rect x="10" width="8" height="8" fill="#7fba00"/><rect y="10" width="8" height="8" fill="#00a4ef"/><rect x="10" y="10" width="8" height="8" fill="#ffb900"/>`,
  package: (p) => stroked(p, 1.8, `<path d="M10 1.5 18 6v9l-8 4.5L2 15V6z"/><path d="M2 6l8 4.5L18 6M10 10.5v9"/><path d="M6 3.8l8 4.5" stroke="#e95420"/>`),
  cup: (p) => stroked(p, 1.8, `<path d="M2 7h13v6a5 5 0 0 1-5 5H7a5 5 0 0 1-5-5z"/><path d="M15 9h1.5a2.5 2.5 0 0 1 0 5H15"/><path d="M6 1.5v2.5M10 1.5v2.5" stroke="${ACCENT}"/>`),
  cupLarge: (p) => stroked(p, 2.6, `<path d="M4 26h48v22a20 20 0 0 1-20 20H24A20 20 0 0 1 4 48z"/><path d="M52 32h6a9 9 0 0 1 0 18h-6"/><path d="M18 4c-4 5 4 9 0 14M30 4c-4 5 4 9 0 14M42 4c-4 5 4 9 0 14" stroke="${ACCENT}"/>`),
  note: (p) => `<path d="M0 4a4 4 0 0 1 4-4h32a4 4 0 0 1 4 4v24l-12 12H4a4 4 0 0 1-4-4z" fill="${p.paper}" stroke="${p.paperEdge}" stroke-width="1.6"/><path d="M28 40V30a2 2 0 0 1 2-2h10" fill="none" stroke="${p.paperEdge}" stroke-width="1.6"/><path d="M8 11h18M8 18h22M8 25h12" stroke="${p.fg}" stroke-width="2" stroke-linecap="round" opacity=".7"/>`,
  code: (p) => stroked(p, 2.2, `<path d="M12 4 2 20l10 16M32 4l10 16-10 16"/><path d="M26 6 18 34" stroke="${ACCENT}"/>`),
  lock: (p) => `${stroked(p, 1.8, `<rect x="0" y="10" width="36" height="22" rx="4"/><path d="M4 32v6h28v-6" opacity=".5"/><path d="M8 10V7a10 7 0 0 1 20 0v3" stroke="${ACCENT}"/>`)}<circle cx="18" cy="21" r="3" fill="${ACCENT}"/>`,
  refresh: (p) => stroked(p, 2, `<path d="M34 14A16 16 0 0 0 6 10"/><path d="M6 2v8h8"/><path d="M6 26a16 16 0 0 0 28 4" stroke="${ACCENT}"/><path d="M34 38v-8h-8" stroke="${ACCENT}"/>`),
  help: (p) => `<circle cx="20" cy="20" r="18" fill="none" stroke="${p.fg}" stroke-width="2"/><path d="M14 15a6 6 0 1 1 8 5.6c-1.4.6-2 1.6-2 3V25" fill="none" stroke="${ACCENT}" stroke-width="2.4" stroke-linecap="round"/><circle cx="20" cy="30.5" r="1.8" fill="${ACCENT}"/>`,
};
