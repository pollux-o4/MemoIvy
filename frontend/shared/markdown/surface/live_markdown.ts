import { syntaxTree } from '@codemirror/language';
import { StateEffect } from '@codemirror/state';
import { Decoration, ViewPlugin, type EditorView, type ViewUpdate } from '@codemirror/view';
import { createListScanner } from './list_markers';
import { AssetWidget, installImageDropHandlers, type ImageHandlers } from './image_tools';

const refreshAfterComposition = StateEffect.define<void>();
const inlineCode = Decoration.mark({ class: 'md-inline-code' });
const codeBlock = Decoration.line({ class: 'md-code-block' });
const hiddenFenceLine = Decoration.line({ class: 'md-hidden-fence-line' });
const imageSource = Decoration.mark({ class: 'md-image-source' });

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
      for (let lineNumber = 1; lineNumber <= view.state.doc.lines; lineNumber++) {
        const line = view.state.doc.line(lineNumber);
        const match = /^!\[([^\]|]*)(?:\|(\d+))?\]\(asset:([a-f\d]{64})\)$/i.exec(line.text.trim());
        if (match) {
          const widget = new AssetWidget({
            description: match[1], src: assetUrls[match[3]], width: Number(match[2]) || undefined,
            hash: match[3], from: line.from, to: line.to, view, handlers: imageHandlers,
          });
          if (line.number === active) ranges.push({ from: line.from, to: line.to, value: imageSource });
          ranges.push(line.number === active ? { from: line.from, to: line.from, value: Decoration.widget({ widget, side: -1 }) } : { from: line.from, to: line.to, value: Decoration.replace({ widget }) });
          continue;
        }
        ranges.push(...listLine(line));
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
    eventObservers: {
      compositionend(_event, view) {
        window.clearTimeout(this.compositionRefresh);
        this.compositionRefresh = window.setTimeout(() => view.dispatch({ effects: refreshAfterComposition.of(undefined) }), 0);
      },
    },
  });
}
