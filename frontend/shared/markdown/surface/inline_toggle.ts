import type { EditorView } from '@codemirror/view';

const PLACEHOLDER = '텍스트';

/**
 * 선택을 서식 기호로 감싸거나 푼다. 기호가 선택 안에 있든, 숨겨진 채 선택 바로 바깥에 있든 같은 키로 해제된다.
 * 바깥 판정은 `**굵게**` 안의 글자에 `*`(기울임)를 눌렀을 때 굵게 기호를 깎지 않도록 같은 기호가 이어지면 제외한다.
 */
export const toggleInlineMark = (left: string, right = left) => (view: EditorView) => {
  if (view.composing) return false;
  const { state } = view;
  const { from, to } = state.selection.main;
  const selected = state.sliceDoc(from, to);
  const inside = selected.length >= left.length + right.length && selected.startsWith(left) && selected.endsWith(right);
  const around = !inside
    && state.sliceDoc(from - left.length, from) === left && state.sliceDoc(to, to + right.length) === right
    && state.sliceDoc(from - left.length - 1, from - left.length) !== left[0]
    && state.sliceDoc(to + right.length, to + right.length + 1) !== right[right.length - 1];
  if (inside) {
    const inner = selected.slice(left.length, selected.length - right.length);
    view.dispatch({ changes: { from, to, insert: inner }, selection: { anchor: from, head: from + inner.length }, scrollIntoView: true });
  } else if (around) {
    view.dispatch({ changes: [{ from: from - left.length, to: from }, { from: to, to: to + right.length }],
      selection: { anchor: from - left.length, head: to - left.length }, scrollIntoView: true });
  } else {
    const body = selected || PLACEHOLDER;
    view.dispatch({ changes: { from, to, insert: `${left}${body}${right}` },
      selection: { anchor: from + left.length, head: from + left.length + body.length }, scrollIntoView: true });
  }
  view.focus?.();
  return true;
};
