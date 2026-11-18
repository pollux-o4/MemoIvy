import { type RefObject, useState } from 'react';
import type { CanvasPoint } from '../model/align';
import { CARD_MENU_SIZE } from '../parts/CardMenu';

/** 카드 메뉴 열림 상태와 위치. 보드 안 화면 좌표에 띄우고 오른쪽·아래 끝에서 잘리지 않게 안쪽으로 당긴다. */
export function useCardMenu(host: RefObject<HTMLElement | null>, select: (ids: string[]) => void) {
  const [menu, setMenu] = useState<{ ids: string[]; at: CanvasPoint } | null>(null);
  const openAt = (ids: string[], client: CanvasPoint) => {
    const board = host.current?.getBoundingClientRect();
    if (!board || !ids.length) return;
    select(ids);
    setMenu({ ids, at: { x: Math.min(client.x - board.left, board.width - CARD_MENU_SIZE.width), y: Math.min(client.y - board.top, board.height - CARD_MENU_SIZE.height) } });
  };
  return {
    menu, openAt, close: () => setMenu(null),
    /** 키보드(Shift+F10·메뉴 키): 첫 선택 카드의 왼쪽 위 근처에 연다. */
    openForSelection: (ids: string[]) => {
      const rect = ids.length ? host.current?.querySelector(`[data-note-id="${ids[0]}"]`)?.getBoundingClientRect() : undefined;
      if (rect) openAt(ids, { x: rect.left + 24, y: rect.top + 40 });
    },
  };
}
