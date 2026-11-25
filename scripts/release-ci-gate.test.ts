import { describe, expect, it } from 'vitest';
import { CI_WORKFLOW, decideGate } from './release-ci-gate.mjs';

const pr = { number: 10 };
const ciRun = { path: CI_WORKFLOW, event: 'push', conclusion: 'success' };

describe('release CI gate', () => {
  it('passes when the merged PR head has the same tree and a successful dev push CI run', () => {
    expect(decideGate({ releaseTree: 't', pr, prHeadTree: 't', prRuns: [ciRun] }).passed).toBe(true);
  });

  it.each([
    ['no merged PR', { releaseTree: 't', pr: undefined, prHeadTree: null, prRuns: [] }],
    ['different code', { releaseTree: 't', pr, prHeadTree: 'other', prRuns: [ciRun] }],
    ['failed CI', { releaseTree: 't', pr, prHeadTree: 't', prRuns: [{ ...ciRun, conclusion: 'failure' }] }],
    ['only a PR run whose jobs were skipped', { releaseTree: 't', pr, prHeadTree: 't', prRuns: [{ ...ciRun, event: 'pull_request' }] }],
    ['another workflow', { releaseTree: 't', pr, prHeadTree: 't', prRuns: [{ ...ciRun, path: '.github/workflows/x.yml' }] }],
  ])('falls back to rerunning CI on %s', (_, input) => {
    expect(decideGate(input).passed).toBe(false);
  });
});
