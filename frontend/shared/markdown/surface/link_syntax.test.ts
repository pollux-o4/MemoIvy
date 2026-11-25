import { ensureSyntaxTree } from '@codemirror/language';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { expect, test } from 'vitest';
import { linkRanges } from './link_syntax';

const links = (doc: string) => {
  const state = EditorState.create({ doc, extensions: [markdown({ base: markdownLanguage })] });
  ensureSyntaxTree(state, doc.length);
  return linkRanges(state).map(({ from, to }) => doc.slice(from, to));
};

test('bracketed text without a destination is plain text', () => {
  expect(links('# [질병청] 2027 계획서')).toEqual([]);
  expect(links('- [ ] 할 일 [메모]')).toEqual([]);
});

test('inline links with a destination are links', () => {
  expect(links('[질병청] 보기 [홈](https://kdca.go.kr)')).toEqual(['[홈](https://kdca.go.kr)']);
});
