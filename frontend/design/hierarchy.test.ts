import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { repo, sourceFiles } from '../test_support/source_files';
import { SECTIONS, sectionOf } from './hierarchy';

const stories = sourceFiles('frontend', /\.stories\.tsx$/);
const titleOf = (file: string) => readFileSync(join(repo, file), 'utf8').match(/title: '([^'/]+\/[^']*)'/)?.[1];

describe('Storybook 계층', () => {
  it.each(stories)('%s 제목이 파일 위치의 묶음에서 시작한다', (file) => {
    const root = sectionOf(file);
    expect(root, '어느 묶음에도 속하지 않는 스토리 파일').toBeDefined();
    expect(titleOf(file)?.split('/')[0]).toBe(root);
  });

  it('모든 묶음이 스토리와 실제 코드 위치를 가진다', () => {
    for (const section of SECTIONS) {
      expect(stories.some((file) => section.from.test(file)), section.root).toBe(true);
      expect(existsSync(join(repo, section.codePath)), section.codePath).toBe(true);
    }
  });

  it('preview 정렬의 최상위 순서가 SECTIONS와 같다', () => {
    const preview = readFileSync(join(repo, '.storybook/preview.ts'), 'utf8');
    const roots = SECTIONS.map((section) => section.root);
    const order = [...preview.matchAll(/^\s{10}'([^']+)',$/gm)].map((match) => match[1]);
    expect(order).toEqual(roots);
  });
});
