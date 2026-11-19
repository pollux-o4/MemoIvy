import { useState } from 'react';
import { useMenu } from '../../shared/menu/useMenu';
import { ConfirmDialog } from '../../shared/ConfirmDialog';
import { Icon } from '../../shared/Icon';
import type { MemoStore } from '../../shared/useMemoivy';

export function NoteMenu({ id, title, store }: { id: string; title: string; store: MemoStore }) {
  const [open, setOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const { root, trigger } = useMenu(open, () => setOpen(false));

  return <div ref={root} className="card-menu-wrap menu-anchor">
    <button ref={trigger} className="icon-button note-more" aria-haspopup="menu" aria-expanded={open} aria-label="더 보기" title="더 보기" onClick={() => setOpen((value) => !value)}><Icon name="more" size={18}/></button>
    {open && <div className="menu-surface menu-popover card-menu" role="menu" aria-label={`${title} 메뉴`}>
      <button className="menu-row" role="menuitem" title="메모에 넣은 이미지와 사용처" onClick={() => { setOpen(false); void store.bridge.openWindow('files').catch(() => undefined); }}><Icon name="files" size={16}/><span>파일</span></button>
      <hr className="menu-divider"/>
      <button className="menu-row destructive" role="menuitem" onClick={() => { setOpen(false); setConfirmDelete(true); }}><Icon name="trash" size={16}/><span>삭제…</span></button>
    </div>}
    {confirmDelete && <ConfirmDialog question="이 메모를 삭제할까요?" subject={title} detail="삭제한 메모는 복구할 수 없습니다." acceptLabel="삭제" onCancel={() => { setConfirmDelete(false); trigger.current?.focus(); }} onAccept={() => { setConfirmDelete(false); void store.removeNote(id).catch(() => undefined); }}/ >}
  </div>;
}
