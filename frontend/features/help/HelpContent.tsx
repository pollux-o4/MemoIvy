import { useLayoutEffect, useRef, useState } from 'react';
import { Icon } from '../../shared/Icon';
import { Segmented } from '../../shared/Segmented';
import { ShortcutKeys } from '../../shared/ShortcutKeys';
import type { HelpTopic } from '../../shared/helpTopic';
import { HelpTopicContent } from './HelpTopicContent';
import { searchHelp, TOPIC_OPTIONS } from './helpSearch';

export function HelpContent({ topic, onTopicChange }: { topic: HelpTopic; onTopicChange: (topic: HelpTopic) => void }) {
  const [query, setQuery] = useState('');
  const [demoTopic, setDemoTopic] = useState<HelpTopic | null>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const scrollArea = useRef<HTMLDivElement>(null);
  const searching = Boolean(query.trim());
  const results = searching ? searchHelp(query) : [];
  // biome-ignore lint/correctness/useExhaustiveDependencies: 새 결과나 탭 본문은 맨 위에서 읽는다.
  useLayoutEffect(() => { if (scrollArea.current) scrollArea.current.scrollTop = 0; }, [query, topic]);
  const changeQuery = (next: string) => { setDemoTopic(null); setQuery(next); };
  const clearSearch = () => { changeQuery(''); searchInput.current?.focus(); };
  return <div className="help-sheet">
    <div className="help-search">
      <Icon name="search" size={16}/>
      <input ref={searchInput} aria-label="도움말 검색" value={query} placeholder="동작, 단축키, 서식 검색"
        onChange={(event) => changeQuery(event.target.value)} onKeyDown={(event) => {
          if (event.key === 'Escape' && query) { event.preventDefault(); clearSearch(); }
        }}/>
      {query && <button type="button" className="icon-button" aria-label="검색어 지우기" onClick={clearSearch}><Icon name="close" size={14}/></button>}
    </div>
    {!searching && <Segmented name="주제" className="help-topics" value={topic} onChange={(next) => { if (next !== topic) setDemoTopic(null); onTopicChange(next); }} options={TOPIC_OPTIONS}/>}
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
      <HelpTopicContent key={topic} topic={topic} searching={searching} demoReady={demoTopic === topic} onDemoMove={() => setDemoTopic(topic)}/>
    </div>
  </div>;
}
