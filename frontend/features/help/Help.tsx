import { useEffect, useId, useState } from 'react';
import { Disclosure } from '../../shared/Disclosure';
import { Reveal } from '../../shared/Reveal';
import { Segmented } from '../../shared/Segmented';
import { ShortcutKeys } from '../../shared/ShortcutKeys';
import type { DesktopBridge } from '../../shared/engine_contract';
import { CANVAS_ESSENTIALS, CANVAS_SHORTCUTS, type Essential, NOTE_ESSENTIALS, NOTE_SHORTCUTS, type ShortcutSection } from '../../shared/shortcuts';
import { WindowHeader } from '../../shared/WindowHeader';
import { HelpDemo } from './HelpDemo';
import { HELP_TOPIC_KEY, type HelpTopic, readHelpTopic, rememberHelpTopic } from '../../shared/helpTopic';
import './help.css';

const TOPICS: Record<HelpTopic, { label: string; essentials: readonly Essential[]; sections: readonly ShortcutSection[] }> = {
  note: { label: '메모', essentials: NOTE_ESSENTIALS, sections: NOTE_SHORTCUTS },
  canvas: { label: '캔버스', essentials: CANVAS_ESSENTIALS, sections: CANVAS_SHORTCUTS },
};
const TOPIC_OPTIONS = (Object.keys(TOPICS) as HelpTopic[]).map((value) => ({ value, label: TOPICS[value].label }));

/**
 * 단축키와 서식: 위에서 아래로 좁혀 읽는다. 주제(메모·캔버스) → 기본 동작 4개 → 전체 목록(묶음별로 펼치기).
 * 처음 주제는 도움말을 연 창의 맥락을 따른다.
 */
export function Help({ bridge, onClose }: { bridge: DesktopBridge; onClose: () => void }) {
  const [topic, setTopic] = useState<HelpTopic>(readHelpTopic);
  useEffect(() => {
    const follow = (event: StorageEvent) => { if (event.key === HELP_TOPIC_KEY) setTopic(readHelpTopic()); };
    window.addEventListener('storage', follow);
    return () => window.removeEventListener('storage', follow);
  }, []);
  const choose = (next: HelpTopic) => { setTopic(next); rememberHelpTopic(next); };
  const { essentials, sections } = TOPICS[topic];
  return <div className="help-screen">
    <WindowHeader className="window-header help-bar" title="단축키와 서식" bridge={bridge} onClose={onClose} controls={[]}/>
    <main className="help-sheet">
      <Segmented name="주제" className="help-topics" value={topic} onChange={choose} options={TOPIC_OPTIONS}/>
      <section className="help-basics" aria-label="기본">
        <h2 className="help-section">기본</h2>
        <ul>{essentials.map(([meaning, keys, demo]) => <li key={meaning} className="help-basic">
          {demo && <HelpDemo kind={demo}/>}
          <strong>{meaning}</strong>
          <ShortcutKeys keys={keys}/>
        </li>)}</ul>
      </section>
      <section className="help-all" aria-label="전체 단축키">
        <h2 className="help-section">전체 단축키</h2>
        {sections.map((section) => <Group key={`${topic}-${section[0]}`} section={section}/>)}
      </section>
    </main>
  </div>;
}

function Group({ section: [title, rows] }: { section: ShortcutSection }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  return <div className={`help-group${open ? ' is-open' : ''}`}>
    <button type="button" className="help-group-head" aria-expanded={open} aria-controls={id} onClick={() => setOpen(!open)}>
      <span>{title}</span><small>{rows.length}</small>
    </button>
    <Disclosure expanded={open} onToggle={() => setOpen(!open)} targetId={id} label={title}/>
    <Reveal open={open} className="help-reveal"><div id={id} className="help-rows">{rows.map(([meaning, syntax, keys]) => <div key={meaning} className="help-row">
      <span>{meaning}</span>
      {syntax && <code className="help-syntax">{syntax}</code>}
      <ShortcutKeys keys={keys}/>
    </div>)}</div></Reveal>
  </div>;
}
