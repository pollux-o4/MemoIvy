import { createBlockScanner } from '../list_item';
import { syntaxTree } from '@codemirror/language';
import { StateEffect } from '@codemirror/state';
import { Decoration, ViewPlugin, type EditorView, type ViewUpdate } from '@codemirror/view';
import { createListScanner } from './list_markers';
import { pendingFenceLine } from './fence_input';
import { AssetWidget, installImageDropHandlers, type ImageHandlers } from './image_tools';
import { parseImageLine } from './image_tools/commands';
import { imageBoundary, keepImageOnOwnLine, splitComposedImageLine } from './image_tools/input';

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
    composingImage?: ReturnType<typeof imageBoundary>;
    finishComposition(view: EditorView) {
      const edits = splitComposedImageLine(view.state, this.composingImage);
      this.composingImage = undefined;
      view.dispatch({ ...edits, effects: refreshAfterComposition.of(undefined) });
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
      const marks = new Set(['HeaderMark', 'EmphasisMark', 'StrongEmphasisMark', 'StrikethroughMark', 'LinkMark', 'ImageMark', 'URL']);
      const ranges: Array<{ from: number; to: number; value: Decoration }> = [{ from: activeLineObj.from, to: activeLineObj.from, value: activeLine }];
      const pendingLine = pendingFenceLine(view.state);
      const listLine = createListScanner({ pendingLine });
      const imageLines = new Set<number>();
      const codeLines = new Set<number>();
      const scanBlock = createBlockScanner({ pendingLine });
      const blocks: Array<{ from: number; to: number; boundaries: Array<{ from: number; to: number; indent: number }> }> = [];
      let block: (typeof blocks)[number] | undefined;
      for (let lineNumber = 1; lineNumber <= view.state.doc.lines; lineNumber++) {
        const line = view.state.doc.line(lineNumber);
        const listDecorations = listLine(line);
        const { fence: boundary, code } = scanBlock(line.text);
        if (boundary?.kind === 'open') {
          block = { from: line.from, to: view.state.doc.length, boundaries: [] };
          blocks.push(block);
        }
        if (code) {
          codeLines.add(lineNumber);
          ranges.push({ from: line.from, to: line.from, value: codeBlock });
          if (boundary && block) block.boundaries.push({ from: line.from, to: line.to, indent: boundary.indent.length });
          if (boundary?.kind === 'close' && block) { block.to = line.to; block = undefined; }
          continue;
        }
        const image = parseImageLine(line.text);
        if (image) {
          imageLines.add(lineNumber);
          // 이미지는 원문을 대체하므로 목록 정렬만 적용한다.
          const alignment = listDecorations[0];
          if (alignment) ranges.push(alignment);
          const widget = new AssetWidget({
            ...image, src: assetUrls[image.hash], from: line.from, to: line.to, view, handlers: imageHandlers,
          });
          if (line.number === active) ranges.push({ from: line.from, to: line.to, value: imageSource });
          ranges.push(line.number === active ? { from: line.from, to: line.from, value: Decoration.widget({ widget, side: -1 }) } : { from: line.from, to: line.to, value: Decoration.replace({ widget }) });
          continue;
        }
        ranges.push(...listDecorations);
      }
      for (const fenced of blocks) {
        if (touchesSelection(fenced.from, fenced.to)) continue;
        for (const boundary of fenced.boundaries) {
          ranges.push({ from: boundary.from + boundary.indent, to: boundary.to, value: hiddenMark });
          ranges.push({ from: boundary.from, to: boundary.from, value: hiddenFenceLine });
        }
      }
      const { from: visibleFrom, to: visibleTo } = view.viewport;
      syntaxTree(view.state).iterate({ from: visibleFrom, to: visibleTo, enter(node) {
        if (codeLines.has(view.state.doc.lineAt(node.from).number)) return;
        if (node.name === 'InlineCode') {
          ranges.push({ from: node.from, to: node.to, value: inlineCode });
        }
        const parent = node.node.parent;
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
