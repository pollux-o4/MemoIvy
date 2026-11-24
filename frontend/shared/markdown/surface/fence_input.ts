import { type EditorState, StateEffect, StateField, type Transaction } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { blockLineAt, fenceOpener } from '../list_item';

/**
 * 코드 펜스 입력 규칙(Notion·Typora와 같은 확정 입력).
 * 사용자가 줄을 여는 펜스 모양(``` 또는 ```언어)으로 만든 순간 그 줄은 '확정 전'이 되고,
 * Enter·Shift+Enter로 확정하면 닫는 펜스를 함께 넣어 문서가 항상 짝이 맞게 남는다.
 * 확정 전에는 스캐너가 그 줄을 블록으로 열지 않아, 입력 중에 아래 블록이 뒤집히지 않는다.
 */
const settle = StateEffect.define<null>();

/** 확정 전 여는 펜스 줄의 시작 위치. 줄 모양이 바뀌거나 확정되면 사라진다(세션 한정 입력 상태). */
export const pendingFence = StateField.define<number | null>({
  create: () => null,
  update(value, transaction) {
    if (transaction.effects.some((effect) => effect.is(settle))) return null;
    if (!transaction.docChanged) return value;
    const typed = typedOpenerLine(transaction);
    if (typed !== undefined) return typed;
    if (value === null) return null;
    const from = transaction.changes.mapPos(value, -1);
    const line = transaction.state.doc.lineAt(from);
    return line.from === from && fenceOpener(line.text) ? from : null;
  },
});

/** 입력으로 커서 줄이 새로 여는 펜스 모양이 됐으면 그 줄의 시작 위치. */
function typedOpenerLine(transaction: Transaction): number | undefined {
  if (!transaction.isUserEvent('input') && !transaction.isUserEvent('paste')) return undefined;
  const line = transaction.state.doc.lineAt(transaction.state.selection.main.head);
  if (!fenceOpener(line.text)) return undefined;
  const before = transaction.startState.doc;
  const start = transaction.changes.invertedDesc.mapPos(line.from, -1);
  const was = before.lineAt(Math.min(start, before.length)).text;
  return fenceOpener(was) ? undefined : line.from;
}

/** 스캐너에 넘길 확정 전 줄 번호(1부터). */
export function pendingFenceLine(state: EditorState): number | undefined {
  const from = state.field(pendingFence, false);
  return from === null || from === undefined ? undefined : state.doc.lineAt(from).number;
}

/** Enter·Shift+Enter: 줄 끝 커서의 여는 펜스를 확정해 빈 줄과 닫는 펜스를 함께 넣는다. */
export function commitCodeFence(view: EditorView): boolean {
  const { state } = view;
  const { main, ranges } = state.selection;
  if (view.composing || ranges.length !== 1 || !main.empty) return false;
  const line = state.doc.lineAt(main.head);
  if (main.head !== line.to) return false;
  // 확정 전 줄은 스캐너가 열지 않으므로, 확정 판정은 보류 없이 원문 그대로 읽는다.
  const boundary = blockLineAt((number) => state.doc.line(number).text, line.number).fence;
  if (boundary?.kind !== 'open') return false;
  const { indent, delimiter } = boundary;
  view.dispatch({
    changes: { from: line.to, insert: `\n${indent}\n${indent}${delimiter}` },
    selection: { anchor: line.to + 1 + indent.length },
    effects: settle.of(null),
    scrollIntoView: true,
    userEvent: 'input',
  });
  return true;
}
