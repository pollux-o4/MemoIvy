import { ensureSyntaxTree } from '@codemirror/language';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { expect, test } from 'vitest';
import { inlineMarks } from './inline_marks';

const marks = (doc: string, kind: 'link' | 'underline') => {
  const state = EditorState.create({ doc, extensions: [markdown({ base: markdownLanguage })] });
  ensureSyntaxTree(state, doc.length);
  return inlineMarks(state).filter((mark) => mark.kind === kind).map(({ from, to }) => doc.slice(from, to));
};

test('bracketed text without a destination is plain text', () => {
  expect(marks('# [질병청] 2027 계획서', 'link')).toEqual([]);
  expect(marks('- [ ] 할 일 [메모]', 'link')).toEqual([]);
});

test('inline links with a destination are links', () => {
  expect(marks('[질병청] 보기 [홈](https://kdca.go.kr)', 'link')).toEqual(['[홈](https://kdca.go.kr)']);
});

test('underline needs a matching <u> pair in the same paragraph', () => {
  expect(marks('a <u>밑 **줄**</u> b <U>대문자</U>', 'underline')).toEqual(['<u>밑 **줄**</u>', '<U>대문자</U>']);
  expect(marks('<u>열기만\n\n닫기만</u>', 'underline')).toEqual([]);
  expect(marks('`<u>코드</u>`', 'underline')).toEqual([]);
});

test('underline tags are reported for hiding', () => {
  const state = EditorState.create({ doc: '<u>x</u>', extensions: [markdown({ base: markdownLanguage })] });
  ensureSyntaxTree(state, 8);
  expect(inlineMarks(state)[0].tags).toEqual([{ from: 0, to: 3 }, { from: 4, to: 8 }]);
});

test('a visible range starting mid-paragraph still sees the opening tag', () => {
  const doc = '<u>첫 줄\n둘째 줄</u>';
  const state = EditorState.create({ doc, extensions: [markdown({ base: markdownLanguage })] });
  ensureSyntaxTree(state, doc.length);
  expect(inlineMarks(state, doc.indexOf('둘째')).map((mark) => mark.kind)).toEqual(['underline']);
});
