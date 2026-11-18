import { syntaxTree } from '@codemirror/language';
import { Decoration, ViewPlugin, type EditorView, type ViewUpdate } from '@codemirror/view';
import { createListScanner } from './list_markers';
import { AssetWidget, installImageDropHandlers, type ImageHandlers } from './image_tools';

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
    destroy() { this.removeDropHandlers(); }
    update(update: ViewUpdate) { if (!update.view.composing && (update.docChanged || update.selectionSet || update.viewportChanged)) this.decorations = this.build(update.view); }
    build(view: EditorView) {
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
      syntaxTree(view.state).iterate({ from: view.visibleRanges[0]?.from ?? 0, to: view.visibleRanges.at(-1)?.to ?? view.state.doc.length, enter(node) {
        if (marks.has(node.name) && (node.name !== 'URL' || node.node.parent?.name === 'Link') && view.state.doc.lineAt(node.from).number !== active && node.to > node.from) {
          // GTK처럼 제목 기호 뒤 공백까지 숨겨 본문이 줄 시작에 붙게 한다.
          // 링크 주소(URL)도 GTK처럼 커서 밖에서는 숨기고 링크 글자만 남긴다.
          const to = node.name === 'HeaderMark' && view.state.doc.sliceString(node.to, node.to + 1) === ' ' ? node.to + 1 : node.to;
          ranges.push({ from: node.from, to, value: hiddenMark });
        }
      } });
      return Decoration.set(ranges.sort((a, b) => a.from - b.from || a.to - b.to).map((item) => item.value.range(item.from, item.to)), true);
    }
  }, { decorations: (plugin) => plugin.decorations });
}
