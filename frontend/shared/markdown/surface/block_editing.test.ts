import { markdown } from '@codemirror/lang-markdown';
import { EditorState, Prec, type TransactionSpec } from '@codemirror/state';
import { keymap, runScopeHandlers, type EditorView } from '@codemirror/view';
import { expect, test } from 'vitest';
import { formattingKeymap } from './formatting_keymap';
import { continueListItem, handleListKey } from './list_markers';

function editor(doc: string, anchor: number) {
  let state = EditorState.create({ doc, selection: { anchor }, extensions: [markdown(), Prec.highest(keymap.of(formattingKeymap()))] });
  return { get state() { return state; }, composing: false,
    dispatch(...specs: TransactionSpec[]) { state = state.update(...specs).state; },
  } as unknown as EditorView;
}

test('soft break keeps marker-width indentation without adding a bullet', () => {
  const view = editor('1. first', 8);
  expect(continueListItem(view)).toBe(true);
  expect(view.state.doc.toString()).toBe('1. first\n   ');
  expect(continueListItem(view)).toBe(true);
  expect(view.state.doc.toString()).toBe('1. first\n   \n   ');
});

test('soft break ignores list-looking code and expired list context', () => {
  for (const doc of ['```\n- code', '- item\n\n\n  text', '- item\nplain\n  text']) {
    const view = editor(doc, doc.length);
    expect(continueListItem(view)).toBe(false);
  }
});

test('renumbering crosses an image continuation and one blank line, but stops at prose', () => {
  const view = editor('1. first\n\n   ![](asset:x)\n2. second\nplain\n3. separate', 8);
  expect(handleListKey(view, 'Enter')).toBe(true);
  expect(view.state.doc.toString()).toBe('1. first\n2. \n\n   ![](asset:x)\n3. second\nplain\n3. separate');
});

test('a new fence above an existing block gets its own closing fence', () => {
  const view = editor('```\n```js\nconst x = 1;\n```', 3);
  expect(formattingKeymap().find(binding => binding.key === 'Enter')!.run(view)).toBe(true);
  expect(view.state.doc.toString()).toBe('```\n\n```\n```js\nconst x = 1;\n```');
});

test.each([['- item', 2], ['- [ ] task', 6], ['12) item', 4]] as const)('Shift-Enter keeps one marker for %s', (doc, indent) => {
  const view = editor(doc, doc.length);
  expect(formattingKeymap().find(binding => binding.key === 'Shift-Enter')!.run(view)).toBe(true);
  expect(view.state.doc.line(2).text.trim()).toBe('');
  expect(view.state.doc.line(2).text.length).toBe(indent);
});

test.each(['```', '~~~', '  ````ts'])('new %s fence completes before an existing block', (fence) => {
  const doc = `${fence}\n\`\`\`\nold code\n\`\`\``;
  const view = editor(doc, fence.length);
  expect(formattingKeymap().find(binding => binding.key === 'Enter')!.run(view)).toBe(true);
  const indent = fence.match(/^ */)![0];
  const delimiter = fence.trim().match(/^[`~]+/)![0];
  expect(view.state.doc.toString()).toBe(`${fence}\n${indent}\n${indent}${delimiter}\n\`\`\`\nold code\n\`\`\``);
});


test('configured keymap runs list and fence commands before Markdown defaults', () => {
  const enter = { key: 'Enter', keyCode: 13, shiftKey: false, altKey: false, ctrlKey: false, metaKey: false } as KeyboardEvent;
  const list = editor('1. first\n\n   ![](x)\n2. second', 8);
  expect(runScopeHandlers(list, enter, 'editor')).toBe(true);
  expect(list.state.doc.toString()).toBe('1. first\n2. \n\n   ![](x)\n3. second');
  const fence = editor('```\n```\nold code\n```', 3);
  expect(runScopeHandlers(fence, enter, 'editor')).toBe(true);
  expect(fence.state.doc.toString()).toBe('```\n\n```\n```\nold code\n```');
  const soft = editor('- item', 6);
  expect(runScopeHandlers(soft, { ...enter, shiftKey: true } as KeyboardEvent, 'editor')).toBe(true);
  expect(soft.state.doc.toString()).toBe('- item\n  ');
});
