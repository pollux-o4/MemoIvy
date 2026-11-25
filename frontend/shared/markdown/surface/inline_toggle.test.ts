import { EditorState, type TransactionSpec } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { expect, test } from 'vitest';
import { toggleInlineMark } from './inline_toggle';

function run(doc: string, anchor: number, head: number, left: string, right?: string) {
  let state = EditorState.create({ doc, selection: { anchor, head } });
  const view = { get state() { return state; }, composing: false,
    dispatch(...specs: TransactionSpec[]) { state = state.update(...specs).state; } } as unknown as EditorView;
  toggleInlineMark(left, right)(view);
  const { from, to } = state.selection.main;
  return { doc: state.doc.toString(), selected: state.sliceDoc(from, to) };
}

test('wraps a selection and keeps the text selected', () => {
  expect(run('a 밑줄 b', 2, 4, '<u>', '</u>')).toEqual({ doc: 'a <u>밑줄</u> b', selected: '밑줄' });
});

test('unwraps when the marks sit just outside the selection (hidden tags)', () => {
  expect(run('a <u>밑줄</u> b', 5, 7, '<u>', '</u>')).toEqual({ doc: 'a 밑줄 b', selected: '밑줄' });
});

test('unwraps when the selection includes the marks', () => {
  expect(run('**굵게**', 0, 6, '**')).toEqual({ doc: '굵게', selected: '굵게' });
});

test('italic inside bold does not eat the bold marks', () => {
  expect(run('**굵게**', 2, 4, '*').doc).toBe('***굵게***');
});

test('empty selection inserts and selects the whole placeholder', () => {
  expect(run('', 0, 0, '<u>', '</u>')).toEqual({ doc: '<u>텍스트</u>', selected: '텍스트' });
});
