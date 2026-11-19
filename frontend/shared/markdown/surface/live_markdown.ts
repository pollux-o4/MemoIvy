import { syntaxTree } from '@codemirror/language';
import { EditorState, StateEffect } from '@codemirror/state';
import { Decoration, ViewPlugin, type EditorView, type ViewUpdate } from '@codemirror/view';
import { createListScanner } from './list_markers';
import { AssetWidget, installImageDropHandlers, type ImageHandlers } from './image_tools';

const refreshAfterComposition = StateEffect.define<void>();
const inlineCode = Decoration.mark({ class: 'md-inline-code' });
const codeBlock = Decoration.line({ class: 'md-code-block' });
const hiddenFenceLine = Decoration.line({ class: 'md-hidden-fence-line' });
const imageSource = Decoration.mark({ class: 'md-image-source' });

const assetLine = /^!\[([^\]|]*)(?:\|(\d+))?\]\(asset:([a-f\d]{64})\)$/i;

function imageBoundary(state: EditorState) {
  const { main, ranges } = state.selection;
  if (ranges.length !== 1 || !main.empty) return;
  const line = state.doc.lineAt(main.head);
  if (main.head !== line.from && main.head !== line.to) return;
  if (assetLine.test(line.text.trim())) return { line, before: main.head === line.from };
}

const keepImageOnOwnLine = EditorState.transactionFilter.of((transaction) => {
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

export function liveMarkdown(assetUrls: Record<string, string>, imageHandlers?: ImageHandlers) {
  const hiddenMark = Decoration.replace({ class: 'cm-mark-hidden' });
  const activeLine = Decoration.line({ class: 'cm-activeLine' });
  return ViewPlugin.fromClass(class {
    decorations;
    constructor(view: EditorView) {
      this.decorations = this.build(view);
      this.removeDropHandlers = installImageDropHandlers(view);
    }
    removeDropHandlers: () => void;
    compositionRefresh?: number;
    composingImage?: ReturnType<typeof imageBoundary>;
    // 입력기가 조합 범위를 넘긴 뒤에만 줄을 나눠 한글 조합을 보존한다.
    finishComposition(view: EditorView) {
      const boundary = this.composingImage;
      this.composingImage = undefined;
      const effects = refreshAfterComposition.of(undefined);
      if (boundary) {
        const { before, line } = boundary;
        const current = view.state.doc.lineAt(Math.min(line.from, view.state.doc.length));
        const attached = before ? current.text.endsWith(line.text) : current.text.startsWith(line.text);
        if (attached && current.length > line.length) {
          const at = before ? current.to - line.length : current.from + line.length;
          const changes = view.state.changes({ from: at, insert: '\n' });
          view.dispatch({ changes, effects, selection: before ? { anchor: at } : view.state.selection.map(changes), userEvent: 'input.type.compose' });
          return;
        }
      }
      view.dispatch({ effects });
    }
    destroy() {
      window.clearTimeout(this.compositionRefresh);
      this.removeDropHandlers();
    }
    update(update: ViewUpdate) {
      if (update.view.composing) {
        if (update.docChanged) this.decorations = this.decorations.map(update.changes);
        return;
      }
      if (update.docChanged || update.selectionSet || update.focusChanged || update.viewportChanged
        || update.transactions.some((transaction) => transaction.effects.some((effect) => effect.is(refreshAfterComposition)))) {
        this.decorations = this.build(update.view);
      }
    }
    build(view: EditorView) {
      const touchesSelection = (from: number, to: number) => view.hasFocus && view.state.selection.ranges.some((range) =>
        range.empty ? range.head >= from && range.head <= to : range.from < to && range.to > from);
      const active = view.state.doc.lineAt(view.state.selection.main.head).number;
      const activeLineObj = view.state.doc.line(active);
      const marks = new Set(['HeaderMark', 'EmphasisMark', 'StrongEmphasisMark', 'StrikethroughMark', 'LinkMark', 'ImageMark', 'CodeMark', 'URL']);
      const ranges: Array<{ from: number; to: number; value: Decoration }> = [{ from: activeLineObj.from, to: activeLineObj.from, value: activeLine }];
      const listLine = createListScanner();
      const imageLines = new Set<number>();
      for (let lineNumber = 1; lineNumber <= view.state.doc.lines; lineNumber++) {
        const line = view.state.doc.line(lineNumber);
        const listDecorations = listLine(line);
        const match = assetLine.exec(line.text.trim());
        if (match) {
          imageLines.add(lineNumber);
          // The image replaces the source range. Keep only the list alignment decoration.
          const alignment = listDecorations[0];
          if (alignment) ranges.push(alignment);
          const widget = new AssetWidget({
            description: match[1], src: assetUrls[match[3]], width: Number(match[2]) || undefined,
            hash: match[3], from: line.from, to: line.to, view, handlers: imageHandlers,
          });
          if (line.number === active) ranges.push({ from: line.from, to: line.to, value: imageSource });
          ranges.push(line.number === active ? { from: line.from, to: line.from, value: Decoration.widget({ widget, side: -1 }) } : { from: line.from, to: line.to, value: Decoration.replace({ widget }) });
          continue;
        }
        ranges.push(...listDecorations);
      }
      const { from: visibleFrom, to: visibleTo } = view.viewport;
      syntaxTree(view.state).iterate({ from: visibleFrom, to: visibleTo, enter(node) {
        if (node.name === 'InlineCode') {
          ranges.push({ from: node.from, to: node.to, value: inlineCode });
        }
        if (node.name === 'FencedCode') {
          const doc = view.state.doc;
          const first = doc.lineAt(Math.max(node.from, visibleFrom)).number;
          const last = doc.lineAt(Math.min(node.to, visibleTo)).number;
          for (let number = first; number <= last; number++) {
            const { from } = doc.line(number);
            ranges.push({ from, to: from, value: codeBlock });
          }
        }
        const parent = node.node.parent;
        if (node.name === 'CodeMark' && parent?.name === 'FencedCode') {
          if (touchesSelection(parent.from, parent.to)) return;
          const line = view.state.doc.lineAt(node.from);
          ranges.push({ from: node.from, to: node.to, value: hiddenMark });
          if (!view.state.sliceDoc(line.from, node.from).trim() && !view.state.sliceDoc(node.to, line.to).trim()) {
            ranges.push({ from: line.from, to: line.from, value: hiddenFenceLine });
          }
          return;
        }
        if (marks.has(node.name) && (node.name !== 'URL' || parent?.name === 'Link') && node.to > node.from) {
          if (imageLines.has(view.state.doc.lineAt(node.from).number)) return;
          const reveal = node.name === 'HeaderMark'
            ? view.state.doc.lineAt(node.from).number === active
            : touchesSelection(parent?.from ?? node.from, parent?.to ?? node.to);
          if (reveal) return;
          // GTK처럼 제목 기호 뒤 공백까지 숨겨 본문이 줄 시작에 붙게 한다.
          // 링크 주소(URL)도 GTK처럼 커서 밖에서는 숨기고 링크 글자만 남긴다.
          const to = node.name === 'HeaderMark' && view.state.doc.sliceString(node.to, node.to + 1) === ' ' ? node.to + 1 : node.to;
          ranges.push({ from: node.from, to, value: hiddenMark });
        }
      } });
      return Decoration.set(ranges.sort((a, b) => a.from - b.from || a.to - b.to).map((item) => item.value.range(item.from, item.to)), true);
    }
  }, {
    decorations: (plugin) => plugin.decorations,
    provide: () => keepImageOnOwnLine,
    eventObservers: {
      compositionstart(_event, view) {
        this.composingImage = imageBoundary(view.state);
      },
      compositionend(_event, view) {
        window.clearTimeout(this.compositionRefresh);
        this.compositionRefresh = window.setTimeout(() => this.finishComposition(view), 0);
      },
    },
  });
}
