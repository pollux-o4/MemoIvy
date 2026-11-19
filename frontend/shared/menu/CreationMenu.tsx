import { useState } from 'react';
import type { CSSProperties } from 'react';
import { Icon } from '../Icon';
import { useMenu } from './useMenu';
import { usePresence } from './usePresence';
import './creation-menu.css';

const MOTION_MS = 280;

/** GTK creation-surface: New 버튼 자리에서 표면이 펼쳐지고(강조색→표면색), + 가 45° 돌아 닫기가 된다. */
export function CreationMenu({ onCreate, disabled = false, projectAllowed = true }: { onCreate: (project: boolean) => void; disabled?: boolean; projectAllowed?: boolean }) {
  const presence = usePresence(MOTION_MS);
  const { root, trigger } = useMenu(presence.open, presence.hide);
  const [origin, setOrigin] = useState<[number, number]>([82, 32]);
  const open = () => {
    if (trigger.current) setOrigin([trigger.current.offsetWidth, trigger.current.offsetHeight]);
    presence.show();
  };
  const choose = (project: boolean) => { presence.dismiss(); onCreate(project); };

  return <div ref={root} className={`creation creation-${presence.state}`} style={{ '--origin-w': `${origin[0]}px`, '--origin-h': `${origin[1]}px` } as CSSProperties}>
    {presence.mounted && <div className="creation-surface" role="menu" aria-label="새로 만들기">
      <div className="creation-choices">
        <button className="menu-row" role="menuitem" disabled={disabled} onClick={() => choose(false)}><Icon name="note" size={18}/><span>메모</span></button>
        <button className="menu-row" role="menuitem" disabled={disabled || !projectAllowed} title={projectAllowed ? undefined : '이 위치에는 프로젝트를 더 만들 수 없어요'} onClick={() => choose(true)}><Icon name="project" size={18}/><span>프로젝트</span></button>
      </div>
    </div>}
    <button ref={trigger} className="primary-button creation-trigger" aria-haspopup="menu" aria-expanded={presence.open} aria-label={presence.open ? '새로 만들기 닫기' : undefined} title="메모 또는 프로젝트 만들기" disabled={disabled} onClick={() => presence.open ? presence.hide() : open()}><span className="creation-plus"><Icon name="plus"/></span><span className="creation-label">New</span></button>
  </div>;
}
