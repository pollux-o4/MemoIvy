import { EditorState } from '@codemirror/state';
import { parseImageLine } from './commands';

export function imageBoundary(state: EditorState) {
  const { main, ranges } = state.selection;
  if (ranges.length !== 1 || !main.empty) return;
  const line = state.doc.lineAt(main.head);
  if (main.head !== line.from && main.head !== line.to) return;
  if (parseImageLine(line.text)) return { line, before: main.head === line.from };
}

export const keepImageOnOwnLine = EditorState.transactionFilter.of((transaction) => {
  if (!transaction.docChanged || !transaction.isUserEvent('input.type') || transaction.isUserEvent('input.type.compose')) return transaction;
  const boundary = imageBoundary(transaction.startState);
  if (!boundary) return transaction;

  const { line, before } = boundary;
  const anchor = before ? line.from : line.to;
  let splitAt: number | undefined;
  transaction.changes.iterChanges((from, to, newFrom, newTo, inserted) => {
    if (from !== anchor || from !== to || !inserted.length) return;
    if (before ? inserted.line(inserted.lines).length : inserted.line(1).length) splitAt = before ? newTo : newFrom;
  });
  if (splitAt === undefined) return transaction;

  const changes = transaction.state.changes({ from: splitAt, insert: '\n' });
  return [transaction, {
    changes,
    selection: before ? { anchor: splitAt } : transaction.newSelection.map(changes),
    sequential: true,
  }];
});

/** Wait until the IME has committed before moving input away from the attachment line. */
export function splitComposedImageLine(state: EditorState, boundary: ReturnType<typeof imageBoundary>) {
  if (!boundary) return;
  const { before, line } = boundary;
  const current = state.doc.lineAt(Math.min(line.from, state.doc.length));
  const attached = before ? current.text.endsWith(line.text) : current.text.startsWith(line.text);
  if (!attached || current.length <= line.length) return;

  const at = before ? current.to - line.length : current.from + line.length;
  const changes = state.changes({ from: at, insert: '\n' });
  return {
    changes,
    selection: before ? { anchor: at } : state.selection.map(changes),
    userEvent: 'input.type.compose',
  };
}
