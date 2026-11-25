import { ICONS } from './icons.mjs';
import { at, cardRow, chip, outArrow, panel, svg, text } from './svg.mjs';
import { ACCENT } from './theme.mjs';

/**
 * README 그래픽 목록. 각 항목은 (palette, context) => SVG 문자열이다.
 * 문구는 여기서만 고친다. README의 alt 문구도 같은 내용을 따른다.
 */

/** 스토어·후원 버튼: 작은 머리글 + 이름 + 바깥 링크 표시. */
const badge = ({ label, title, icon, width, iconAt = [18, 15] }) => (palette) => svg({ width, height: 48 },
  panel(palette, { width, height: 48, radius: 10 })
  + at(...iconAt, ICONS[icon](palette))
  + text(palette, { x: 48, y: 20, value: label, size: 10, weight: 500, mono: true, spacing: 1.2 })
  + text(palette, { x: 48, y: 36, value: title, size: 15, color: palette.fg, weight: 600 })
  + outArrow(width - 24, 28));

/** 강점 카드: 아이콘 · 번호 · 제목 · 두 줄 설명 · 강조선. */
const featureCard = (palette, { no, icon, title, lines }) => (width) =>
  at(28, 28, ICONS[icon](palette))
  + text(palette, { x: +(width - 24).toFixed(1), y: 44, value: no, size: 13, mono: true, anchor: 'end', spacing: 1.4 })
  + text(palette, { x: 28, y: 134, value: title, size: 27, color: palette.fg, weight: 700 })
  + lines.map((line, i) => text(palette, { x: 28, y: 172 + i * 28, value: line })).join('')
  + `<rect x="28" y="214" width="28" height="3" rx="1.5" fill="${ACCENT}"/>`;

const FEATURES = [
  { no: '01', icon: 'note', title: '메모마다 독립 창', lines: ['필요한 메모만 띄워 옆에 붙여 둡니다.', '쓰는 즉시 자동으로 저장됩니다.'] },
  { no: '02', icon: 'code', title: '무료 · 오픈소스', lines: ['광고·구독 없이 모든 기능을 씁니다.', '코드는 MIT 라이선스로 공개합니다.'] },
  { no: '03', icon: 'lock', title: '계정 없이, 내 컴퓨터에', lines: ['가입·서버 없이 로컬에 저장하고,', '인터넷 없이도 그대로 씁니다.'] },
];

const tips = (palette, { helpImage }) => svg({ width: 1200, height: 300 }, cardRow(palette, { width: 1200, height: 300, cards: [
  () => at(28, 28, ICONS.refresh(palette))
    + text(palette, { x: 28, y: 118, value: '업데이트는 자동', size: 26, color: palette.fg, weight: 700 })
    + text(palette, { x: 28, y: 154, value: ['앱이 켜져 있으면 ', { text: '종료한 뒤', accent: true }, ' 적용됩니다.'], size: 18 })
    + text(palette, { x: 28, y: 200, value: '바로 받기', size: 14, weight: 500, mono: true })
    + text(palette, { x: 28, y: 232, value: 'Windows  Store 라이브러리 → 업데이트 받기', size: 17, color: palette.fg })
    + chip(palette, { x: 28, y: 276, value: 'sudo snap refresh memoivy' }),
  (width) => at(28, 28, ICONS.help(palette))
    + text(palette, { x: 28, y: 118, value: '단축키·서식은 앱 안에서', size: 26, color: palette.fg, weight: 700 })
    + text(palette, { x: 28, y: 154, value: [{ text: 'F1', mono: true }, ' 또는 캔버스 아래 ', { text: '?', accent: true }, ' — 움직이는 예시로 봅니다.'], size: 18 })
    + `<image x="28" y="180" width="${width - 56}" height="${Math.round((width - 56) * helpImage.height / helpImage.width)}" href="data:image/png;base64,${helpImage.base64}"/>`,
] }));

const support = (palette) => svg({ width: 1200, height: 150 },
  panel(palette, { width: 1200, height: 150 })
  + at(40, 38, ICONS.cupLarge(palette))
  + text(palette, { x: 140, y: 66, value: 'MemoIvy가 도움이 됐다면', size: 26, color: palette.fg, weight: 700 })
  + text(palette, { x: 140, y: 102, value: '커피 한 잔으로 다음 기능을 응원해 주세요.' })
  + at(900, 47, `<rect width="260" height="56" rx="28" fill="${ACCENT}"/>${text(palette, { x: 32, y: 36, value: 'Buy Me a Coffee', size: 20, color: '#ffffff', weight: 700 })}${outArrow(214, 34, '#ffffff', 2.2)}`));

export const ASSETS = {
  'ms-store': badge({ label: 'WINDOWS', title: 'Microsoft Store', icon: 'windows', width: 212 }),
  'snap-store': badge({ label: 'LINUX', title: 'Snap Store', icon: 'package', width: 180, iconAt: [17, 13] }),
  coffee: badge({ label: 'SUPPORT', title: 'Buy Me a Coffee', icon: 'cup', width: 212, iconAt: [17, 13] }),
  features: (palette) => svg({ width: 1200, height: 236 }, cardRow(palette, { width: 1200, height: 236, cards: FEATURES.map((item) => featureCard(palette, item)) })),
  tips,
  support,
};
