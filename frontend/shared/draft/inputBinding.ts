import type { KeyboardEvent } from 'react';
import type { Draft } from './useDraft';

type Field = HTMLInputElement | HTMLTextAreaElement;

/**
 * 초안을 input·textarea 속성으로 잇는다. 저장은 입력 칸의 현재 값을 읽어 입력 직후 Enter·포커스 이동이 렌더보다 빨라도 마지막 글자를 잃지 않는다.
 * Esc는 되돌리고, Enter(여러 줄이면 Ctrl/⌘+Enter)는 편집을 마친다.
 */
export function bindInput(draft: Draft, { multiline = false }: { multiline?: boolean } = {}) {
  return {
    value: draft.value,
    onChange: (event: { target: { value: string } }) => draft.change(event.target.value),
    onCompositionStart: () => draft.compose(true),
    onCompositionEnd: (event: { currentTarget: { value: string } }) => draft.compose(false, event.currentTarget.value),
    onBlur: (event: { currentTarget: { value: string } }) => draft.finish(event.currentTarget.value),
    onKeyDown: (event: KeyboardEvent<Field>) => {
      event.stopPropagation(); // 입력 중 캔버스 단축키(삭제·방향키 등)가 동작하지 않게 한다.
      if (event.nativeEvent.isComposing) return;
      if (event.key === 'Escape') { event.currentTarget.value = draft.revert(); event.currentTarget.blur(); }
      else if (event.key === 'Enter' && (!multiline || event.ctrlKey || event.metaKey)) { event.preventDefault(); event.currentTarget.blur(); }
    },
  };
}
