import { type DragEvent, useState } from 'react';
import type { OutlineRow } from '../../model/links/types';

export type Place = 'before' | 'after';

/**
 * 가지 개요 행 끌어 놓기 상태. 행 위·아래 절반으로 앞·뒤를 정하고, 놓으면 onMove(성공하면 onMoved)를 부른다.
 * 뿌리 행은 끌거나 그 옆에 놓을 수 없다.
 */
export function useOutlineDrag({ labelOf, onMove, onMoved }: {
  labelOf: (id: string) => string; onMove: (id: string, target: string, place: Place) => boolean; onMoved: (id: string) => void;
}) {
  const [dragging, setDragging] = useState<string | undefined>(undefined);
  const [drop, setDrop] = useState<{ id: string; place: Place } | undefined>(undefined);
  const placeOf = (event: DragEvent<HTMLElement>): Place => {
    const box = event.currentTarget.getBoundingClientRect();
    return event.clientY < box.top + box.height / 2 ? 'before' : 'after';
  };
  const end = () => { setDragging(undefined); setDrop(undefined); };
  return {
    /** 놓을 자리(li)에 붙인다. */
    target: (row: OutlineRow) => ({
      className: `${dragging === row.id ? 'is-dragging' : ''}${drop?.id === row.id ? ` drop-${drop.place}` : ''}`,
      onDragOver: (event: DragEvent<HTMLElement>) => {
        if (!dragging || dragging === row.id || row.depth === 0) return;
        event.preventDefault();
        const place = placeOf(event);
        if (drop?.id !== row.id || drop.place !== place) setDrop({ id: row.id, place });
      },
      onDragLeave: (event: DragEvent<HTMLElement>) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDrop(undefined); },
      onDrop: (event: DragEvent<HTMLElement>) => {
        event.preventDefault();
        if (dragging && dragging !== row.id && row.depth > 0 && onMove(dragging, row.id, placeOf(event))) onMoved(dragging);
        end();
      },
    }),
    /** 끄는 손잡이(행 버튼)에 붙인다. */
    source: (row: OutlineRow) => ({
      draggable: row.depth > 0,
      onDragStart: (event: DragEvent<HTMLElement>) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('text/plain', labelOf(row.id)); setDragging(row.id); },
      onDragEnd: end,
    }),
  };
}
