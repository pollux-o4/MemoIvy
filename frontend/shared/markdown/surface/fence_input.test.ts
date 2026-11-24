import { EditorSelection, EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { expect, test } from 'vitest';
import { createBlockScanner } from '../list_item';
import { commitCodeFence, pendingFence, pendingFenceLine } from './fence_input';

const BELOW = '\n```js\nold code\n```\n아래 글';

function typing(doc: string, at: number) {
  let state = EditorState.create({ doc, selection: { anchor: at }, extensions: [pendingFence] });
  const view = { get state() { return state; }, composing: false, dispatch: (spec: Parameters<EditorState['update']>[0]) => { state = state.update(spec).state; } };
  const type = (text: string) => {
    const head = state.selection.main.head;
    state = state.update({ changes: { from: head, insert: text }, selection: { anchor: head + text.length }, userEvent: 'input.type' }).state;
  };
  return { view: view as unknown as EditorView, type, get state() { return state; } };
}

/** 렌더러와 같은 방식으로 코드로 칠해질 줄 번호를 구한다. */
const codeLines = (state: EditorState) => {
  const scan = createBlockScanner({ pendingLine: pendingFenceLine(state) });
  return Array.from({ length: state.doc.lines }, (_, index) => scan(state.doc.line(index + 1).text).code ? index + 1 : 0).filter(Boolean);
};

test('typing a fence above a block keeps the block below intact until Enter', () => {
  const editor = typing(`위 글${BELOW}`, 3);
  editor.type('\n``');
  expect(codeLines(editor.state)).toEqual([3, 4, 5]);
  editor.type('`');
  expect(pendingFenceLine(editor.state)).toBe(2);
  expect(codeLines(editor.state)).toEqual([3, 4, 5]);
  expect(commitCodeFence(editor.view)).toBe(true);
  expect(editor.state.doc.toString()).toBe(`위 글\n\`\`\`\n\n\`\`\`${BELOW}`);
  expect(pendingFenceLine(editor.state)).toBeUndefined();
  expect(codeLines(editor.state)).toEqual([2, 3, 4, 5, 6, 7]);
});

test('an uncommitted fence stays inert after the cursor leaves its line', () => {
  const editor = typing(`위 글${BELOW}`, 3);
  editor.type('\n```');
  const state = editor.state.update({ selection: EditorSelection.cursor(0) }).state;
  expect(pendingFenceLine(state)).toBe(2);
  expect(codeLines(state)).toEqual([3, 4, 5]);
});

test('a fence followed by prose is plain text and Enter does not convert it', () => {
  const line = '```백틱을 추가하면 아래가 사라짐';
  const editor = typing(`${line}${BELOW}`, line.length);
  expect(codeLines(editor.state)).toEqual([2, 3, 4]);
  expect(commitCodeFence(editor.view)).toBe(false);
});

test('a language word is part of the committed fence', () => {
  const editor = typing('', 0);
  editor.type('```ts');
  expect(commitCodeFence(editor.view)).toBe(true);
  expect(editor.state.doc.toString()).toBe('```ts\n\n```');
});
