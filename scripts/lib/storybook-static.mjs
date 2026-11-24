// 테스트 공용: 현재 코드로 Storybook 정적 빌드 → 빈 포트(127.0.0.1)에 띄운다.
// 개발 서버(감시자)는 파일 수 한계로 옛 코드를 보여 줄 수 있어 검사는 항상 새 빌드로 한다.
import { spawn } from 'node:child_process';
import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize, resolve } from 'node:path';

export const root = resolve(import.meta.dirname, '../..');
// STORYBOOK_OUT으로 빌드 폴더를 나누면 여러 검사를 동시에 돌려도 서로 덮어쓰지 않는다.
const out = process.env.STORYBOOK_OUT ? resolve(process.env.STORYBOOK_OUT) : join(root, 'storybook-static');
export const bin = (name) => join(root, 'node_modules', '.bin', name);

export const run = (command, commandArgs) => new Promise((done) => {
  const child = spawn(command, commandArgs, { cwd: root, stdio: 'inherit', env: process.env });
  child.on('exit', (code, signal) => done(code ?? (signal ? 1 : 0)));
});

const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.woff2': 'font/woff2', '.wasm': 'application/wasm' };

/** build=false면 기존 storybook-static을 쓴다. 반환값: { url, close }. */
export async function serveStorybook({ build }) {
  if (build) {
    const code = await run(bin('storybook'), ['build', '--quiet', '-o', out]);
    if (code) process.exit(code);
  }
  if (!existsSync(join(out, 'iframe.html'))) { console.error('storybook-static가 없습니다. --no-build 없이 실행하세요.'); process.exit(1); }
  const server = createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url ?? '/', 'http://x').pathname);
    let file = normalize(join(out, path));
    if (!file.startsWith(out)) { response.writeHead(403).end(); return; }
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' });
    createReadStream(file).pipe(response);
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  // Wayland 세션에서도 브라우저 창이 사용자 화면에 뜨지 않게 한다(테스트 브라우저는 헤드리스).
  delete process.env.WAYLAND_DISPLAY;
  return { url: `http://127.0.0.1:${server.address().port}`, close: () => server.close() };
}
