import { useEffect, useRef } from 'react';
import type { RefObject } from 'react';

const NAVIGATION = ['ArrowDown', 'ArrowUp', 'Home', 'End'];

/**
 * 팝오버 메뉴 공통 동작(GTK popover): 열리면 첫 항목 포커스, 바깥 누름·Esc로 닫기(Esc는 트리거로 포커스 복귀),
 * 방향키·Home·End로 사용 가능한 항목 순환. 열림 상태는 호출자가 가진다.
 */
export function useMenu<Root extends HTMLElement = HTMLDivElement, Trigger extends HTMLElement = HTMLButtonElement>(open: boolean, onClose: () => void): { root: RefObject<Root | null>; trigger: RefObject<Trigger | null> } {
  const root = useRef<Root>(null);
  const trigger = useRef<Trigger>(null);
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    if (!open) return;
    const items = () => [...(root.current?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? [])];
    // 마우스로 연 경우 첫 항목 강조를 남기지 않는다.
    items()[0]?.focus({ preventScroll: true, focusVisible: false } as FocusOptions);
    const outside = (event: PointerEvent) => { if (!root.current?.contains(event.target as Node)) close.current(); };
    const keys = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); close.current(); trigger.current?.focus(); return; }
      if (!NAVIGATION.includes(event.key)) return;
      const rows = items();
      const current = rows.indexOf(document.activeElement as HTMLElement);
      const index = event.key === 'Home' ? 0 : event.key === 'End' ? rows.length - 1 : (current + (event.key === 'ArrowUp' ? -1 : 1) + rows.length) % rows.length;
      event.preventDefault();
      rows[index]?.focus();
    };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', keys);
    return () => { document.removeEventListener('pointerdown', outside); document.removeEventListener('keydown', keys); };
  }, [open]);
  return { root, trigger };
}
