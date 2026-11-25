import { syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';
import { Decoration, type DecorationSet, type EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';

type SyntaxNode = ReturnType<typeof syntaxTree>['topNode'];
type Range = { from: number; to: number };

/** 파서 노드만으로 정해지지 않는 인라인 서식. `tags`는 커서 밖에서 숨길 표시 기호다. */
export type InlineMark = Range & { kind: 'link' | 'underline'; tags: Range[] };

/**
 * 링크 판정의 단일 기준. 파서는 정의 없는 `[글자]`도 Link 노드로 만들지만,
 * 앱이 여는 링크는 `[이름](주소)`뿐이라 주소가 있는 Link만 링크로 본다(CommonMark: 정의 없는 대괄호는 글자).
 * shortcut: 참조 링크(`[이름][라벨]`, `[라벨]: 주소`)는 열기·서식 모두 미지원, 사용 요구가 생기면 정의 해석을 추가.
 */
export function isMarkdownLink(node: SyntaxNode | null | undefined): boolean {
  return node?.name === 'Link' && node.getChild('URL') !== null;
}

const UNDERLINE_OPEN = /^<u>$/i;
const UNDERLINE_CLOSE = /^<\/u>$/i;

/**
 * 보이는 범위를 한 번 순회해 인라인 서식을 찾는다(O(노드 수)).
 * 밑줄은 Markdown 표준이 없어 HTML `<u>…</u>`로 저장하고, 같은 문단 안에서 짝이 맞을 때만 인정한다.
 */
export function inlineMarks(state: EditorState, from = 0, to = state.doc.length): InlineMark[] {
  const marks: InlineMark[] = [];
  let open: { tag: Range; block: number } | undefined;
  const tree = syntaxTree(state);
  // 보이는 범위가 문단 중간에서 시작해도 그 문단의 여는 `<u>`를 보도록 최상위 블록 시작부터 훑는다.
  let block = tree.resolveInner(from, 1);
  while (block.parent && block.parent.name !== 'Document') block = block.parent;
  tree.iterate({ from: Math.min(from, block.from), to, enter(node) {
    if (node.name === 'Link') {
      if (isMarkdownLink(node.node)) marks.push({ from: node.from, to: node.to, kind: 'link', tags: [] });
      return false;
    }
    if (node.name !== 'HTMLTag') return;
    const tag = state.sliceDoc(node.from, node.to);
    const block = node.node.parent?.from ?? -1;
    if (UNDERLINE_OPEN.test(tag)) open = { tag: { from: node.from, to: node.to }, block };
    else if (UNDERLINE_CLOSE.test(tag) && open?.block === block) {
      marks.push({ from: open.tag.from, to: node.to, kind: 'underline', tags: [open.tag, { from: node.from, to: node.to }] });
      open = undefined;
    }
  } });
  return marks;
}

const STYLE: Record<InlineMark['kind'], Decoration> = {
  link: Decoration.mark({ class: 'md-link' }),
  underline: Decoration.mark({ class: 'md-underline' }),
};

/** 인라인 서식 class를 단다. 원문 모드에서도 같은 기준을 쓰도록 live 장식과 분리한다. */
export const inlineStyling = ViewPlugin.fromClass(class {
  decorations: DecorationSet;
  constructor(view: EditorView) { this.decorations = this.build(view); }
  update(update: ViewUpdate) {
    if (update.docChanged || update.viewportChanged || syntaxTree(update.state) !== syntaxTree(update.startState)) {
      this.decorations = this.build(update.view);
    }
  }
  build(view: EditorView) {
    const { from, to } = view.viewport;
    return Decoration.set(inlineMarks(view.state, from, to).map((mark) => STYLE[mark.kind].range(mark.from, mark.to)), true);
  }
}, { decorations: (plugin) => plugin.decorations });
