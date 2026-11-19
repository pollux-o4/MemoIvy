const { spawn } = require('node:child_process');
const http = require('node:http');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const children = new Set();
let stopping = false;
function start(command, args, env = {}) {
  const child = spawn(command, args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } });
  children.add(child);
  child.on('exit', () => children.delete(child));
  child.on('error', error => { console.error(error.message); stop(1); });
  return child;
}
function stop(code = 0) {
  if (stopping) return;
  stopping = true;
  for (const child of children) child.kill('SIGTERM');
  process.exitCode = code;
}
process.on('SIGINT', () => stop());
process.on('SIGTERM', () => stop());
function ready(url, timeout = 30000) {
  const end = Date.now() + timeout;
  return new Promise((resolve, reject) => {
    function poll() {
      const request = http.get(url, response => { response.resume(); resolve(); });
      request.on('error', () => Date.now() > end ? reject(new Error('화면 서버 시작 시간 초과')) : setTimeout(poll, 150));
      request.setTimeout(1000, () => request.destroy());
    }
    poll();
  });
}
(async () => {
  const build = start('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--locked']);
  const code = await new Promise(resolve => build.on('exit', resolve));
  if (code !== 0 || stopping) return stop(1);
  const vite = start(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js')]);
  vite.on('exit', code => stop(code || 0));
  await ready('http://127.0.0.1:5173');
  if (stopping) return;
  const electron = start(require('electron'), ['.'], { MEMOIVY_RENDERER_URL: 'http://127.0.0.1:5173' });
  electron.on('exit', code => stop(code || 0));
})().catch(error => { console.error(error.message); stop(1); });
