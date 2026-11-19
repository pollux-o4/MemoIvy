// Storybook 상호작용·접근성 검사: 새 정적 빌드에 test-storybook을 돌린다.
// 사용: node scripts/test-stories.mjs [--no-build] [test-storybook 인자...]
import { bin, run, serveStorybook } from './lib/storybook-static.mjs';

const args = process.argv.slice(2);
const storybook = await serveStorybook({ build: !args.includes('--no-build') });
const code = await run(bin('test-storybook'), ['--url', storybook.url, ...args.filter((arg) => arg !== '--no-build')]);
storybook.close();
process.exit(code);
