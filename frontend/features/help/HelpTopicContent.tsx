import { useId, useState } from 'react';
import { Disclosure } from '../../shared/Disclosure';
import { Reveal } from '../../shared/Reveal';
import { ShortcutKeys } from '../../shared/ShortcutKeys';
import type { HelpTopic } from '../../shared/helpTopic';
import type { ShortcutSection } from '../../shared/shortcuts';
import { HelpDemo } from './HelpDemo';
import { TOPICS } from './helpSearch';

export function HelpTopicContent({ topic, searching, demoReady, onDemoMove }: {
  topic: HelpTopic; searching: boolean; demoReady: boolean; onDemoMove: () => void;
}) {
  const { essentials, sections } = TOPICS[topic];
  return <div className="help-topic-content help-appear" hidden={searching}>
    <section className={`help-basics${demoReady ? ' is-demo-ready' : ''}`} aria-label="기본">
      <div className="help-basics-heading">
        <h2 className="help-section">기본</h2>
        {essentials.some(([, , demo]) => demo) && <span className="help-demo-hint">마우스를 올리면 재생</span>}
      </div>
      <ul>{essentials.map(([meaning, keys, demo]) => <li key={meaning} className="help-basic" onMouseMove={demo ? onDemoMove : undefined}>
        {demo && <><HelpDemo kind={demo}/><span className="help-demo-play" aria-hidden="true">▶</span></>}
        <strong>{meaning}</strong>
        <ShortcutKeys keys={keys}/>
      </li>)}</ul>
    </section>
    <section className="help-all" aria-label="전체 단축키">
      <h2 className="help-section">전체 단축키</h2>
      {sections.map((section) => <Group key={section[0]} section={section}/>)}
    </section>
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
