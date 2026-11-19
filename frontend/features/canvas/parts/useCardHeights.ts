import { type RefObject, useLayoutEffect, useState } from 'react';
import { CARD_HEIGHT } from '../model/align';

const CARD = '.canvas-note[data-note-id]';

export function useCardHeights(world: RefObject<HTMLElement | null>): Record<string, number> {
  const [heights, setHeights] = useState<Record<string, number>>({});
  useLayoutEffect(() => {
    const root = world.current;
    if (!root) return;
    const cards = new Map<HTMLElement, string>();
    const resize = new ResizeObserver((entries) => {
      // offsetHeight는 카메라 배율을 제외한 세계 좌표다. 바뀐 카드만 읽는다.
      const measured = entries.flatMap(({ target }) => {
        const card = target as HTMLElement;
        const id = cards.get(card);
        return id === undefined ? [] : [{ id, height: card.offsetHeight }];
      });
      if (!measured.length) return;
      setHeights((current) => {
        let next = current;
        for (const { id, height } of measured) {
          if ((next[id] ?? CARD_HEIGHT) === height) continue;
          if (next === current) next = { ...current };
          if (height === CARD_HEIGHT) delete next[id];
          else next[id] = height;
        }
        return next;
      });
    });
    const add = (node: Node) => {
      if (!(node instanceof HTMLElement) || !node.matches(CARD) || node.dataset.noteId === undefined) return;
      cards.set(node, node.dataset.noteId);
      resize.observe(node);
      return node.dataset.noteId;
    };
    const children = new MutationObserver((records) => {
      const removed = new Set<string>();
      for (const { removedNodes, addedNodes } of records) {
        for (const node of removedNodes) {
          const card = node as HTMLElement;
          const id = cards.get(card);
          if (id === undefined) continue;
          cards.delete(card);
          resize.unobserve(card);
          removed.add(id);
        }
        for (const node of addedNodes) {
          const id = add(node);
          if (id !== undefined) removed.delete(id);
        }
      }
      if (!removed.size) return;
      setHeights((current) => {
        let next = current;
        for (const id of removed) {
          if (!(id in next)) continue;
          if (next === current) next = { ...current };
          delete next[id];
        }
        return next;
      });
    });
    children.observe(root, { childList: true });
    for (const card of root.querySelectorAll(`:scope > ${CARD}`)) add(card);
    return () => { children.disconnect(); resize.disconnect(); };
  }, [world]);
  return heights;
}
