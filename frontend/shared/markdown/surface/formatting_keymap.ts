import type { EditorView } from '@codemirror/view';
import { continueListItem, handleListKey } from './list_markers';
import { commitCodeFence } from './fence_input';
import { toggleInlineMark as wrap } from './inline_toggle';

export function formattingKeymap() {
  const linePrefix = (prefix: string) => (view: EditorView) => {
    if (view.composing) return false;
    const line = view.state.doc.lineAt(view.state.selection.main.head);
    const text = line.text;
    const existing = /^(?:#{1,2}\s+|[-*+]\s+|\d+[.)]\s+|[-*+]\s+\[[ x/]]\s+)/.exec(text)?.[0] ?? '';
    const insert = text.startsWith(prefix) ? text.slice(prefix.length) : `${prefix}${text.slice(existing.length)}`;
    view.dispatch({ changes: { from: line.from, to: line.to, insert } }); view.focus(); return true;
  };
  const mark = (regexp: RegExp, replacement: (match: RegExpExecArray) => string) => (view: EditorView) => {
    if (view.composing) return false;
    const line = view.state.doc.lineAt(view.state.selection.main.head);
    const match = regexp.exec(line.text);
    if (!match) return false;
    view.dispatch({ changes: { from: line.from, to: line.to, insert: replacement(match) } }); view.focus(); return true;
  };
  return [
    { key: 'Mod-b', run: wrap('**') }, { key: 'Mod-i', run: wrap('*') }, { key: 'Mod-u', run: wrap('<u>', '</u>') }, { key: 'Mod-Shift-x', run: wrap('~~') },
    { key: 'Mod-1', run: linePrefix('# ') }, { key: 'Mod-2', run: linePrefix('## ') },
    { key: 'Mod-Shift-7', run: linePrefix('1. ') }, { key: 'Mod-Shift-8', run: linePrefix('- ') }, { key: 'Mod-Shift-9', run: linePrefix('- [ ] ') },
    { key: 'Mod-k', run: wrap('[', '](https://example.com)') },
    { key: 'Enter', run: commitCodeFence },
    { key: 'Shift-Enter', run: (view: EditorView) => commitCodeFence(view) || continueListItem(view) },
    ...['Enter', 'Backspace', 'Delete', 'Tab', 'Shift-Tab', 'Home', 'ArrowLeft'].map((key) => ({ key, run: (view: EditorView) => handleListKey(view, key) })),
    { key: 'Mod-Enter', run: mark(/^(\s*[-*+]\s+\[)([ x/])(\])/, (match) => `${match[1]}${match[2].toLowerCase() === 'x' ? ' ' : 'x'}${match[3]}`) },
  ];
}
