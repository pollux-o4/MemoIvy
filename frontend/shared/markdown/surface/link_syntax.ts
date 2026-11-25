import { syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';
import { Decoration, type DecorationSet, type EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import type { SyntaxNode } from '@lezer/common';

/**
 * 링크 판정의 단일 기준. 파서는 정의 없는 `[글자]`도 Link 노드로 만들지만,
 * 앱이 여는 링크는 `[이름](주소)`뿐이라 주소가 있는 Link만 링크로 본다(CommonMark: 정의 없는 대괄호는 글자).
 * shortcut: 참조 링크(`[이름][라벨]`, `[라벨]: 주소`)는 열기·서식 모두 미지원, 사용 요구가 생기면 정의 해석을 추가.
 */
export function isMarkdownLink(node: SyntaxNode | null | undefined): boolean {
  return node?.name === 'Link' && node.getChild('URL') !== null;
}

const linkText = Decoration.mark({ class: 'md-link' });

export function linkRanges(state: EditorState, from = 0, to = state.doc.length) {
  const ranges: Array<{ from: number; to: number }> = [];
  syntaxTree(state).iterate({ from, to, enter(node) {
    if (node.name !== 'Link') return;
    if (isMarkdownLink(node.node)) ranges.push({ from: node.from, to: node.to });
    return false;
  } });
  return ranges;
}

/** 실제 링크에만 밑줄 class를 단다. 원문 모드에서도 같은 기준을 쓰도록 live 장식과 분리한다. */
export const linkStyling = ViewPlugin.fromClass(class {
  decorations: DecorationSet;
  constructor(view: EditorView) { this.decorations = this.build(view); }
  update(update: ViewUpdate) {
    if (update.docChanged || update.viewportChanged || syntaxTree(update.state) !== syntaxTree(update.startState)) {
      this.decorations = this.build(update.view);
    }
  }
  build(view: EditorView) {
    const { from, to } = view.viewport;
    return Decoration.set(linkRanges(view.state, from, to).map((range) => linkText.range(range.from, range.to)));
  }
}, { decorations: (plugin) => plugin.decorations });
