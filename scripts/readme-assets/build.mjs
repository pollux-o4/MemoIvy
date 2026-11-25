#!/usr/bin/env node
// README 그래픽 생성: node scripts/readme-assets/build.mjs (mise run readme:assets)
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { ASSETS } from './assets.mjs';
import { THEMES } from './theme.mjs';

const root = path.resolve(import.meta.dirname, '../..');
const out = path.join(root, '.github/readme');

/** 캔버스 도구 막대의 ? 버튼 강조 이미지. 스토어 스크린샷 03이 원본이다. */
const HELP_CROP = { source: 'distribution/snap/visuals/screenshots/03.png', crop: [360, 62, 766, 773], scale: 2, mark: [376, 34, 56] };

function buildHelpImage() {
  const { source, crop: [w, h, x, y], scale, mark: [mx, my, size] } = HELP_CROP;
  const file = path.join(out, 'help-button.png');
  execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-i', path.join(root, source), '-vf',
    `crop=${w}:${h}:${x}:${y},scale=${w * scale}:${h * scale}:flags=lanczos,drawbox=x=${mx}:y=${my}:w=${size}:h=${size}:color=0xff6b35:t=5`,
    '-frames:v', '1', file]);
  return { base64: readFileSync(file).toString('base64'), width: w * scale, height: h * scale };
}

export function renderAll(context) {
  return Object.entries(ASSETS).flatMap(([name, render]) =>
    Object.entries(THEMES).map(([theme, palette]) => ({ file: `${name}-${theme}.svg`, content: render(palette, context) })));
}

if (process.argv[1] === import.meta.filename) {
  for (const { file, content } of renderAll({ helpImage: buildHelpImage() })) writeFileSync(path.join(out, file), content);
  console.log(`README 그래픽 ${Object.keys(ASSETS).length * Object.keys(THEMES).length}개 생성: ${path.relative(root, out)}`);
}
