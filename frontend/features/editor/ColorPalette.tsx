import { useState } from 'react';
import { Icon } from '../../shared/Icon';
import { NOTE_COLORS, noteColor } from '../../shared/model';
import type { Note } from '../../shared/engine_contract';


export function ColorPalette({ color, onChange }: { color: Note['color']; onChange: (color: string) => void }) {
  const [open, setOpen] = useState(false);
  return <div className="palette-wrap">
    <button className="icon-button palette-trigger" aria-label="메모 색상" title="메모 색상" onClick={() => setOpen(!open)}><Icon name="palette"/></button>
    {open && <div className="palette-popover menu-surface" role="menu" aria-label="메모 색상">
      {NOTE_COLORS.map((value) => {
        const info = noteColor(value);
        const selected = color === value;
        return <button key={value} role="menuitemradio" aria-checked={selected} className="swatch" style={{ backgroundColor: info.accent }} title={info.label} aria-label={info.label} onClick={() => { onChange(value); setOpen(false); }}>
        </button>;
      })}
    </div>}
  </div>;
}
