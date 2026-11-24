import { useState } from 'react';
import { MorphMenu } from '../../shared/menu/MorphMenu';
import { useMenu } from '../../shared/menu/useMenu';
import { Icon } from '../../shared/Icon';
import { WindowHeader } from '../../shared/WindowHeader';
import type { Note } from '../../shared/engine_contract';
import type { MemoStore } from '../../shared/useMemoivy';
import { ColorPalette } from './ColorPalette';

export function EditorToolbar({ note, store, onClose, raw, onToggleRaw, onDelete, onColor }: {
  note: Note;
  store: MemoStore;
  onClose: () => void;
  raw: boolean;
  onToggleRaw: () => void;
  onDelete: () => void;
  onColor: (color: string) => void;
}) {
  const [menu, setMenu] = useState(false);
  const { root, trigger } = useMenu(menu, () => setMenu(false));
  const actions = <div className="editor-tools menu-anchor" ref={root}>
    <MorphMenu tone="quiet" label="이동" title="캔버스나 메모 목록으로 이동" trigger={() => <Icon name="open"/>} choices={[
      { key: 'canvas', icon: 'canvas', label: '캔버스에서 보기', onSelect: () => void store.bridge.openWindow('canvas', note.id) },
      { key: 'library', icon: 'note', label: '메모 목록', onSelect: () => void store.bridge.openWindow('library') },
    ]}/>
    <ColorPalette color={note.color} onChange={onColor}/>
    <button ref={trigger} className="icon-button" title="더보기" aria-label="더보기" aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu(!menu)}><Icon name="more"/></button>
    {menu && <div className="menu-popover menu-surface" role="menu" aria-label="메모 메뉴">
      <button className="menu-row" role="menuitem" onClick={() => { onToggleRaw(); setMenu(false); }}>원문 {raw ? '숨기기' : '보기'} <kbd className="menu-hint">Ctrl ⇧ M</kbd></button>
      <button className="menu-row" role="menuitem" onClick={() => { void store.bridge.openWindow('files'); setMenu(false); }}>파일</button>
      <button className="menu-row destructive" role="menuitem" onClick={() => { setMenu(false); onDelete(); }}>삭제</button>
      <button className="menu-row" role="menuitem" onClick={() => { setMenu(false); window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1' })); }}>단축키·서식 도움말 <kbd className="menu-hint">F1</kbd></button>
    </div>}
  </div>;

  return <WindowHeader
      className="editor-topbar"
      bridge={store.bridge}
      onClose={onClose}
      title={<span className="editor-title">{note.canvas.project ? '프로젝트' : '메모'}</span>}
      actions={actions}
    />;
}
