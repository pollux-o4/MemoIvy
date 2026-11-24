import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Icon, type IconName } from '../Icon';
import { useMenu } from './useMenu';
import { usePresence } from './usePresence';
import './morph-menu.css';

const MOTION_MS = 280;

export type MorphChoice = { key: string; icon: IconName; label: string; onSelect: () => void; disabled?: boolean; title?: string };

/**
 * 버튼 자리에서 표면이 위→아래로 펼쳐지는 메뉴(GTK creation-surface). 버튼 크기를 원점으로 clip-path가 늘어나고
 * 버튼 색(`--morph-from`)이 표면색으로 이어진다. 열기·닫기·키보드는 useMenu·usePresence가 맡고, 여기서는 모양만 정한다.
 * `tone`은 버튼 바탕(강조 버튼·아이콘 버튼)만 바꾼다.
 */
export function MorphMenu({ label, title, tone = 'accent', trigger, choices, disabled = false, className = '' }: {
  label: string; title: string; tone?: 'accent' | 'quiet'; trigger: (open: boolean) => ReactNode; choices: readonly MorphChoice[]; disabled?: boolean; className?: string;
}) {
  const presence = usePresence(MOTION_MS);
  const { root, trigger: triggerRef } = useMenu(presence.open, presence.hide);
  const [origin, setOrigin] = useState<[number, number]>([32, 32]);
  const open = () => {
    if (triggerRef.current) setOrigin([triggerRef.current.offsetWidth, triggerRef.current.offsetHeight]);
    presence.show();
  };
  const choose = (choice: MorphChoice) => { presence.dismiss(); choice.onSelect(); };

  return <div ref={root} className={`morph morph-${tone} morph-${presence.state} ${className}`} style={{ '--origin-w': `${origin[0]}px`, '--origin-h': `${origin[1]}px` } as CSSProperties}>
    {presence.mounted && <div className="morph-surface" role="menu" aria-label={label}>
      <div className="morph-choices">
        {choices.map((choice) => <button key={choice.key} className="menu-row" role="menuitem" disabled={disabled || choice.disabled} title={choice.title} onClick={() => choose(choice)}><Icon name={choice.icon} size={18}/><span>{choice.label}</span></button>)}
      </div>
    </div>}
    <button ref={triggerRef} className={`morph-trigger ${tone === 'accent' ? 'primary-button' : 'icon-button'}`} aria-haspopup="menu" aria-expanded={presence.open} aria-label={presence.open ? `${label} 닫기` : tone === 'quiet' ? label : undefined} title={title} disabled={disabled} onClick={() => presence.open ? presence.hide() : open()}>{trigger(presence.open)}</button>
  </div>;
}
