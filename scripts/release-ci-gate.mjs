#!/usr/bin/env node
// 출시 검사 관문(OPS-005): main 출시 커밋과 같은 코드(트리)가 병합 PR 머리 커밋의 dev 푸시 CI를 이미 통과했으면 검사를 다시 돌리지 않는다.
// dev→main PR은 같은 커밋을 다시 검사하지 않아(ci.yml) pull_request 실행은 작업을 건너뛴 채 성공으로 남으므로 근거로 쓰지 않는다.
// 확인할 수 없으면 passed=false를 내고, 출시 워크플로가 CI 전체를 다시 실행한다(실패 시 안전 쪽).
import { execFileSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const CI_WORKFLOW = '.github/workflows/ci.yml';

/** 판단 규칙만 담는다. 입력은 GitHub·git에서 읽은 값이다. */
export function decideGate({ releaseTree, pr, prHeadTree, prRuns }) {
  if (!pr) return { passed: false, reason: '출시 커밋에 연결된 병합 PR이 없다' };
  if (prHeadTree !== releaseTree) return { passed: false, reason: `PR #${pr.number} 머리와 출시 커밋의 코드가 다르다` };
  const ok = prRuns.some((run) => run.path === CI_WORKFLOW && run.event === 'push' && run.conclusion === 'success');
  if (!ok) return { passed: false, reason: `PR #${pr.number} 머리 커밋에 성공한 dev 푸시 CI가 없다` };
  return { passed: true, reason: `PR #${pr.number}에서 같은 코드가 CI를 통과했다` };
}

function gh(path) {
  return JSON.parse(execFileSync('gh', ['api', path], { encoding: 'utf8' }));
}

function tree(rev) {
  try {
    return execFileSync('git', ['rev-parse', `${rev}^{tree}`], { encoding: 'utf8' }).trim();
  } catch {
    return null;
  }
}

function main() {
  const repo = process.env.GITHUB_REPOSITORY;
  const sha = process.env.GITHUB_SHA;
  const pr = gh(`repos/${repo}/commits/${sha}/pulls`).find((p) => p.merged_at && p.base.ref === 'main');
  const headSha = pr?.head.sha;
  // 병합 시 머리 브랜치가 자동 삭제되면 체크아웃에 그 커밋이 없다. PR 참조는 GitHub이 계속 보관한다.
  if (pr && !tree(headSha)) execFileSync('git', ['fetch', '--no-tags', 'origin', `refs/pull/${pr.number}/head`], { stdio: 'inherit' });
  const decision = decideGate({
    releaseTree: tree(sha),
    pr,
    prHeadTree: headSha ? tree(headSha) : null,
    prRuns: headSha ? gh(`repos/${repo}/actions/runs?head_sha=${headSha}&per_page=100`).workflow_runs : [],
  });
  console.log(`${decision.passed ? '통과 확인' : 'CI 재실행'}: ${decision.reason}`);
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `passed=${decision.passed}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
