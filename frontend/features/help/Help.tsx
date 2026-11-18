import { CANVAS_SHORTCUTS, NOTE_SHORTCUTS, type ShortcutSection } from '../../shared/shortcuts';
import type { DesktopBridge } from '../../shared/engine_contract';
import { WindowHeader } from '../../shared/WindowHeader';
import './help.css';

/** GTK '단축키와 서식' 창: 두 열(메모 · 캔버스), 뜻 · 문법 · 키캡. */
export function Help({ bridge, onClose }: { bridge: DesktopBridge; onClose: () => void }) {
  return <div className="help-screen">
    <WindowHeader className="window-header help-bar" title="단축키와 서식" bridge={bridge} onClose={onClose} controls={[]}/>
    <main className="help-sheet">
      <Column sections={NOTE_SHORTCUTS}/>
      <Column sections={CANVAS_SHORTCUTS}/>
    </main>
  </div>;
}

function Column({ sections }: { sections: readonly ShortcutSection[] }) {
  return <div className="help-column">{sections.map(([title, rows]) => <section key={title} className="help-group" aria-label={title}>
    <h2 className="help-section">{title}</h2>
    {rows.map(([meaning, syntax, keys]) => <div key={meaning} className={`help-row${syntax ? ' has-syntax' : ''}`}>
      <span>{meaning}</span>
      {syntax && <code className="help-syntax">{syntax}</code>}
      <span className="help-keys">{keys.map((key) => <kbd key={key} className="help-key">{key}</kbd>)}</span>
    </div>)}
  </section>)}</div>;
}
