import { type RefObject, useLayoutEffect, useState } from 'react';
import { CARD_HEIGHT } from '../model/align';

export function useCardHeights(world: RefObject<HTMLElement | null>): Record<string, number> {
  const [heights, setHeights] = useState<Record<string, number>>({});
  useLayoutEffect(() => {
    const root = world.current;
    if (!root) return;
    let cards = new Map<HTMLElement, string>();
    const resize = new ResizeObserver((entries) => {
      // offsetHeight는 카메라 배율을 제외한 세계 좌표다. 바뀐 카드만 읽는다.
      const measured = entries.flatMap(({ target }) => {
        const card = target as HTMLElement;
        const id = cards.get(card);
        return id === undefined ? [] : [{ id, height: card.offsetHeight }];
      });
      setHeights((current) => {
        const next = { ...current };
        let changed = false;
        for (const { id, height } of measured) {
          if ((next[id] ?? CARD_HEIGHT) === height) continue;
          changed = true;
          if (height === CARD_HEIGHT) delete next[id];
          else next[id] = height;
        }
        return changed ? next : current;
      });
    });
    const watch = () => {
      const next = new Map<HTMLElement, string>();
      for (const card of root.querySelectorAll<HTMLElement>(':scope > .canvas-note[data-note-id]')) {
        const id = card.dataset.noteId;
        if (id === undefined) continue;
        next.set(card, id);
        if (!cards.has(card)) resize.observe(card);
      }
      for (const card of cards.keys()) if (!next.has(card)) resize.unobserve(card);
      cards = next;
      const ids = new Set(next.values());
      setHeights((current) => {
        const stale = Object.keys(current).filter((id) => !ids.has(id));
        if (!stale.length) return current;
        const heights = { ...current };
        for (const id of stale) delete heights[id];
        return heights;
      });
    };
    const children = new MutationObserver(watch);
    children.observe(root, { childList: true });
    watch();
    return () => { children.disconnect(); resize.disconnect(); };
  }, [world]);
  return heights;
}
