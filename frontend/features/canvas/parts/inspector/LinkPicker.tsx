import { type Ref, useImperativeHandle, useState } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { useMenu } from '../../../../shared/menu/useMenu';
import { canvasTitle } from '../../model/labels';

/** 다음 메모 고르기. 앱 메뉴 표면으로 그린다(UI-001 기본 컨트롤 금지). 후보마다 메모 색 점으로 카드와 맞춰 찾게 한다. */
export function LinkPicker({ candidates, onPick, ref }: { candidates: Note[]; onPick: (id: string) => void; ref: Ref<() => void> }) {
  const [open, setOpen] = useState(false);
  useImperativeHandle(ref, () => () => setOpen(true), []);
  const { root, trigger } = useMenu(open, () => setOpen(false));
  return <div ref={root} className="inspector-add-link menu-anchor">
    <button ref={trigger} className="inspector-add-trigger" aria-keyshortcuts="Control+L" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
      <Icon name="plus" size={13}/><span>다음 메모 연결</span><Icon name="chevron" size={13}/>
    </button>
    {open && <div className="menu-surface menu-popover inspector-link-menu" role="menu" aria-label="연결할 다음 메모">
      {candidates.map((item) => <button key={item.id} className="menu-row" role="menuitem" title={canvasTitle(item)}
        onClick={() => { setOpen(false); onPick(item.id); trigger.current?.focus(); }}>
        <i className={`inspector-dot ${item.color}`} aria-hidden="true"/><span>{canvasTitle(item)}</span>
      </button>)}
    </div>}
  </div>;
}
