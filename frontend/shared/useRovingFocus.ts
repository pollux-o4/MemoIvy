import { type RefObject, useCallback, useEffect } from 'react';

/** 현재 DOM 순서의 활성 버튼만 탐색한다. 반환한 조회 함수로 동일한 범위를 Tab 트랩에도 쓴다. */
export function useRovingFocus(containerRef: RefObject<HTMLElement | null>, {
  orientation, loop,
}: { orientation: 'horizontal' | 'vertical'; loop: boolean }) {
  const buttons = useCallback(() => Array.from(containerRef.current?.querySelectorAll<HTMLButtonElement>('button') ?? [])
    .filter((button) => !button.matches(':disabled, [aria-disabled="true"]')
      && !button.closest('[hidden], [inert], [aria-hidden="true"]')
      && button.getClientRects().length > 0
      && getComputedStyle(button).visibility === 'visible'), [containerRef]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      const previous = orientation === 'horizontal' ? 'ArrowLeft' : 'ArrowUp';
      const next = orientation === 'horizontal' ? 'ArrowRight' : 'ArrowDown';
      if (![previous, next, 'Home', 'End'].includes(event.key)) return;
      const items = buttons();
      const active = container.ownerDocument.activeElement;
      const index = active instanceof HTMLButtonElement ? items.indexOf(active) : -1;
      if (index < 0) return;
      let target = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : index + (event.key === next ? 1 : -1);
      target = loop ? (target + items.length) % items.length : Math.max(0, Math.min(target, items.length - 1));
      event.preventDefault();
      event.stopPropagation();
      items[target].focus();
    };
    container.addEventListener('keydown', onKey);
    return () => container.removeEventListener('keydown', onKey);
  }, [containerRef, orientation, loop, buttons]);
  return buttons;
}
