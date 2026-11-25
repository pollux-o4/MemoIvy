import { expect, test } from 'vitest';
import { renderAll } from './build.mjs';
import { THEMES } from './theme.mjs';

const files = renderAll({ helpImage: { base64: 'AA==', width: 720, height: 124 } });

test('every asset renders once per theme as standalone SVG', () => {
  expect(files).toHaveLength(6 * Object.keys(THEMES).length);
  for (const { content } of files) expect(content).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg".*<\/svg>$/s);
});

test('each theme file uses only its own palette', () => {
  for (const { file, content } of files) {
    const other = file.endsWith('-dark.svg') ? THEMES.light : THEMES.dark;
    expect(content).not.toContain(`fill="${other.fg}"`);
  }
});

test('text is escaped', () => {
  expect(files.find(({ file }) => file === 'features-light.svg')?.content).not.toMatch(/>[^<]*&(?!amp;|lt;|gt;|quot;)[^<]*</);
});
