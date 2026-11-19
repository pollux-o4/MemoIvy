import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { Icon } from '../../shared/Icon';
import { Disclosure } from '../../shared/Disclosure';
import { Reveal } from '../../shared/Reveal';
import { Segmented } from '../../shared/Segmented';
import { ShortcutKeys } from '../../shared/ShortcutKeys';
import type { DesktopBridge } from '../../shared/engine_contract';
import type { ShortcutSection } from '../../shared/shortcuts';
import { searchHelp, TOPIC_OPTIONS, TOPICS } from './helpSearch';
import { WindowHeader } from '../../shared/WindowHeader';
import { HelpDemo } from './HelpDemo';
import { HELP_TOPIC_KEY, type HelpTopic, readHelpTopic, rememberHelpTopic } from '../../shared/helpTopic';
import './help.css';

/**
 * 단축키와 서식: 위에서 아래로 좁혀 읽는다. 주제(메모·캔버스) → 기본 동작 4개 → 전체 목록(묶음별로 펼치기).
 * 처음 주제는 도움말을 연 창의 맥락을 따른다.
 */
export function Help({ bridge, initialTopic, onClose }: { bridge: DesktopBridge; initialTopic?: HelpTopic; onClose: () => void }) {
  const [topic, setTopic] = useState<HelpTopic>(() => initialTopic ?? readHelpTopic());
  const [query, setQuery] = useState('');
  const [demoReady, setDemoReady] = useState(false);
  const searchInput = useRef<HTMLInputElement>(null);
  const scrollArea = useRef<HTMLDivElement>(null);
  const searching = Boolean(query.trim());
  const results = searching ? searchHelp(query) : [];
  // biome-ignore lint/correctness/useExhaustiveDependencies: 새 결과나 탭 본문은 맨 위에서 읽는다.
  useLayoutEffect(() => { if (scrollArea.current) scrollArea.current.scrollTop = 0; }, [query, topic]);
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useLayoutEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
  }, []);
  useEffect(() => {
    if (initialTopic) return;
    const follow = (event: StorageEvent) => { if (event.key === HELP_TOPIC_KEY) setTopic(readHelpTopic()); };
    window.addEventListener('storage', follow);
    return () => window.removeEventListener('storage', follow);
  }, [initialTopic]);
  const choose = (next: HelpTopic) => { if (next !== topic) setDemoReady(false); setTopic(next); if (!initialTopic) rememberHelpTopic(next); };
  const { essentials, sections } = TOPICS[topic];
  const changeQuery = (next: string) => { setDemoReady(false); setQuery(next); };
  const clearSearch = () => { changeQuery(''); searchInput.current?.focus(); };
  const content = <div className="help-sheet">
    <div className="help-search">
      <Icon name="search" size={16}/>
      <input ref={searchInput} aria-label="도움말 검색" value={query} placeholder="동작, 단축키, 서식 검색"
        onChange={(event) => changeQuery(event.target.value)} onKeyDown={(event) => {
          if (event.key === 'Escape' && query) { event.preventDefault(); clearSearch(); }
        }}/>
      {query && <button type="button" className="icon-button" aria-label="검색어 지우기" onClick={clearSearch}><Icon name="close" size={14}/></button>}
    </div>
    {!searching && <Segmented name="주제" className="help-topics" value={topic} onChange={choose} options={TOPIC_OPTIONS}/>}
    <div className="help-content" ref={scrollArea}>
      {searching && <section className="help-results help-appear" aria-label="검색 결과">
        <p role="status" className="help-section">{results.length ? `검색 결과 ${results.length}개` : '검색 결과가 없어요'}</p>
        <ul>{results.map(({ topic: resultTopic, label, meaning, syntax, keys }) => <li key={`${resultTopic}-${meaning}`} className="help-result">
          <span className="help-result-topic">{label}</span>
          <span>{meaning}</span>
          {syntax && <code className="help-syntax">{syntax}</code>}
          <ShortcutKeys keys={keys}/>
        </li>)}</ul>
      </section>}
      <div key={topic} className="help-topic-content help-appear" hidden={searching}>
        <section className={`help-basics${demoReady ? ' is-demo-ready' : ''}`} aria-label="기본">
          <div className="help-basics-heading">
            <h2 className="help-section">기본</h2>
            {essentials.some(([, , demo]) => demo) && <span className="help-demo-hint">마우스를 올리면 재생</span>}
          </div>
          <ul>{essentials.map(([meaning, keys, demo]) => <li key={meaning} className="help-basic" onMouseMove={demo ? () => setDemoReady(true) : undefined}>
            {demo && <><HelpDemo kind={demo}/><span className="help-demo-play" aria-hidden="true">▶</span></>}
            <strong>{meaning}</strong>
            <ShortcutKeys keys={keys}/>
          </li>)}</ul>
        </section>
        <section className="help-all" aria-label="전체 단축키">
          <h2 className="help-section">전체 단축키</h2>
          {sections.map((section) => <Group key={`${topic}-${section[0]}`} section={section}/>)}
        </section>
      </div>
    </div>
  </div>;
  if (!initialTopic) return <div className="help-screen">
    <WindowHeader className="window-header help-bar" title="단축키와 서식" bridge={bridge} onClose={onClose} controls={[]}/>
    {content}
  </div>;
  const close = () => { dialog.current?.close(); onClose(); };
  return <dialog ref={dialog} className="help-dialog" aria-labelledby={titleId} onCancel={(event) => { event.preventDefault(); close(); }}>
    <div className="help-screen">
      <header className="help-dialog-header">
        <h2 id={titleId}>단축키와 서식</h2>
        <button type="button" className="icon-button" aria-label="도움말 닫기" onClick={close}><Icon name="close" size={16}/></button>
      </header>
      {content}
    </div>
  </dialog>;
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
