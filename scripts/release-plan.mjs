#!/usr/bin/env node
// 출시 계획 SSoT: package.json 버전에서 태그를 정하고, 이미 출시된 버전이면 실패한다(OPS-005).
// CI(dev→main PR)의 버전 확인과 main 출시 워크플로가 같은 규칙을 쓴다.
import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';

const { version } = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const tag = `v${version}`;
const released = execFileSync('git', ['tag', '--list', tag], { encoding: 'utf8' }).trim() !== '';

if (released) {
  console.error(`${tag}은 이미 출시됐다. dev에서 package.json 버전을 올린 뒤 main에 병합한다.`);
  process.exit(1);
}
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\ntag=${tag}\n`);
console.log(tag);
