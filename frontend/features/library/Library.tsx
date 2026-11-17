import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { Note } from '../../shared/engine_contract';
import { Disclosure } from '../../shared/Disclosure';
import { Icon } from '../../shared/Icon';
import { checklist, plainText, projectPath, titleOf, visibleNoteOrder, NOTE_PALETTE } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';

const colors: Record<string, string> = Object.fromEntries(Object.entries(NOTE_PALETTE).map(([name, value]) => [name, value.accent]));

export function Library({ store, onOpenNote, onOpenCanvas, onCreate }: { store: MemoStore; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void; onCreate: (project: boolean) => void }) {
  const [query, setQuery] = useState('');
  const [mode, setMode] = useState<'projects' | 'recent'>('projects');
  const [openNew, setOpenNew] = useState(false);
  const [expanded, setExpanded] = useState<string[]>(() => (store.snapshot?.preferences?.expandedProjects as string[] | undefined) ?? []);
  const projects = store.notes.filter((note) => note.canvas.project);
  const matches = useMemo(() => visibleNoteOrder(store.notes, query), [store.notes, query]);
  const searching = query.trim().length > 0;
  const displayed = matches;
  const topLevel = store.notes.filter((note) => !note.canvas.parent);

  useEffect(() => {
    setExpanded((store.snapshot?.preferences?.expandedProjects as string[] | undefined) ?? []);
  }, [store.snapshot?.preferences?.expandedProjects]);

  const toggleExpanded = (id: string) => {
    const next = expanded.includes(id) ? expanded.filter((item) => item !== id) : [...expanded, id];
    setExpanded(next);
    void store.request('preference', { key: 'expandedProjects', value: next }).catch(() => setExpanded(expanded));
  };
  const create = (project: boolean) => { setOpenNew(false); onCreate(project); };

  return <div className="library-screen">
    <header className="window-header">
      <div className="window-controls"><button aria-label="창 닫기" title="닫기" onClick={() => store.bridge.windowAction('close')}><Icon name="close"/></button><button aria-label="최소화" title="최소화" onClick={() => store.bridge.windowAction('minimize')}><Icon name="minimize"/></button><button aria-label="최대화" title="최대화" onClick={() => store.bridge.windowAction('maximize')}><Icon name="maximize"/></button></div>
      <span className="window-title">MemoIvy</span>
      <div className="header-actions"><button className="icon-button canvas-launch" title="캔버스 열기" aria-label="캔버스 열기" onClick={() => onOpenCanvas()}><Icon name="canvas"/></button><div className="new-wrap"><button className="primary-button" onClick={() => setOpenNew(!openNew)}><Icon name="plus"/> New</button>{openNew && <div className="new-menu" role="menu"><button role="menuitem" onClick={() => create(false)}><span className="menu-glyph">✎</span><span>메모<small>새 메모 작성</small></span><kbd>Ctrl N</kbd></button><button role="menuitem" onClick={() => create(true)}><span className="menu-glyph">▧</span><span>프로젝트<small>메모를 모아 관리</small></span></button></div>}</div></div>
    </header>
    <main className="library-main">
      <label className="search-field"><Icon name="search" size={17}/><input id="library-search" autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="메모 검색" aria-label="메모 검색"/>{query && <button aria-label="검색 지우기" onClick={() => setQuery('')}><Icon name="close" size={15}/></button>}</label>
      <div className="count-bar"><span className="count">메모 {store.notes.length}개{projects.length > 0 && ` · 프로젝트 ${projects.length}개`}</span>{!searching && projects.length > 0 && <div className="segmented" role="tablist" aria-label="목록 보기"><button role="tab" aria-selected={mode === 'projects'} className={mode === 'projects' ? 'selected' : ''} onClick={() => setMode('projects')}>프로젝트별</button><button role="tab" aria-selected={mode === 'recent'} className={mode === 'recent' ? 'selected' : ''} onClick={() => setMode('recent')}>최근</button></div>}</div>
      {searching || mode === 'recent' ? <section className="note-list" aria-label="메모 목록">{displayed.map((note) => <NoteCard key={note.id} note={note} path={searching ? projectPath(note, store.notes) : []} onOpen={() => onOpenNote(note.id)}/>)}</section> : <section className="note-list" aria-label="메모와 프로젝트">{topLevel.map((note) => note.canvas.project ? <ProjectSection key={note.id} project={note} notes={store.notes} expandedIds={expanded} onToggle={toggleExpanded} onOpenNote={onOpenNote} onOpenCanvas={onOpenCanvas}/> : <NoteCard key={note.id} note={note} path={[]} onOpen={() => onOpenNote(note.id)}/>)}</section>}
      {searching && displayed.length === 0 && <div className="empty-state"><p>검색 결과가 없어요. 다른 단어로 찾아보세요.</p></div>}
      {!store.notes.length && <div className="empty-state"><p>첫 메모를 작성해 보세요.</p><button className="primary-button" onClick={() => create(false)}><Icon name="plus"/> 새 메모</button></div>}
    </main>
  </div>;
}

function ProjectSection({ project, notes, expandedIds, onToggle, onOpenNote, onOpenCanvas }: { project: Note; notes: Note[]; expandedIds: string[]; onToggle: (id: string) => void; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void }) {
  const expanded = expandedIds.includes(project.id);
  const children = notes.filter((note) => note.canvas.parent === project.id);
  const count = children.length;
  const tasks = checklist([project.text, ...descendants(project.id, notes).map((note) => note.text)].join('\n'));
  const regionId = `section-${project.id}`;
  return <article className="project-section" style={{ '--note-accent': colors[project.color] ?? colors.yellow } as React.CSSProperties}>
    <div className="project-heading"><div className="card-stripe"/><button className="project-title" onClick={() => onOpenNote(project.id)}><strong><span>{titleOf(project)}</span>{tasks.total > 0 && <span className={`task-chip${tasks.done === tasks.total ? ' complete' : ''}`} title={`할 일 ${tasks.total}개 중 ${tasks.done}개 완료`}>{tasks.done}/{tasks.total}</span>}</strong><span>메모 {count}개</span>{tasks.total > 0 && <span className="project-progress" role="progressbar" aria-label={`${titleOf(project)} 할 일 진행률`} aria-valuemin={0} aria-valuemax={tasks.total} aria-valuenow={tasks.done}><i style={{ width: `${tasks.done / tasks.total * 100}%` }}/></span>}</button><button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${titleOf(project)} 캔버스 열기`} onClick={() => onOpenCanvas(project.id)}><Icon name="enter" size={17}/></button><button className="icon-button note-more" aria-label={`${titleOf(project)} 메뉴`} title={`${titleOf(project)} 메뉴`} onClick={() => onOpenNote(project.id)}><Icon name="more" size={18}/></button></div>
    {children.length > 0 && <><div className={`project-contents${expanded ? ' expanded' : ''}`} id={regionId} aria-hidden={!expanded} inert={!expanded}><div className="project-content-inner">{children.map((child) => child.canvas.project ? <ProjectSection key={child.id} project={child} notes={notes} expandedIds={expandedIds} onToggle={onToggle} onOpenNote={onOpenNote} onOpenCanvas={onOpenCanvas}/> : <NoteCard key={child.id} note={child} path={[titleOf(project)]} onOpen={() => onOpenNote(child.id)}/>)}</div></div><Disclosure expanded={expanded} onToggle={() => onToggle(project.id)} targetId={regionId} label={titleOf(project)}/></>}
  </article>;
}

function NoteCard({ note, path, onOpen }: { note: Note; path: string[]; onOpen: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const [needsReveal, setNeedsReveal] = useState(false);
  const previewRef = useRef<HTMLDivElement>(null);
  const body = note.text.split('\n').slice(1).join('\n').trim();
  const summary = plainText(body);
  const task = checklist(note.text);
  const id = `preview-${note.id}`;
  useLayoutEffect(() => {
    const element = previewRef.current;
    if (!element) { setNeedsReveal(false); return; }
    const measure = () => setNeedsReveal(element.scrollHeight > parseFloat(getComputedStyle(element).lineHeight) * 2 + 1);
    measure();
    const observer = new ResizeObserver(measure); observer.observe(element);
    return () => observer.disconnect();
  }, [summary, expanded]);
  return <article className={`note-card card ${note.color}`}>
    <div className="card-row"><div className="card-stripe"/><button className="card-open" onClick={onOpen} aria-label={`${titleOf(note)} 메모 열기`}><div className="card-content">
      {path.length > 0 && <div className="card-location">{path.join(' / ')}</div>}
      <div className="card-title-row"><strong className="card-title">{titleOf(note)}</strong>{task.total > 0 && <span className={`task-chip${task.done === task.total ? ' complete' : ''}`} title={`할 일 ${task.total}개 중 ${task.done}개 완료`}>{task.done}/{task.total}</span>}</div>
      {summary && <div ref={previewRef} className={`preview${needsReveal && !expanded ? ' has-more' : ''}${expanded ? ' is-expanded' : ''}`} id={id} aria-hidden={false}>{summary}</div>}
    </div></button><button className="icon-button note-more" title={`${titleOf(note)} 메뉴`} aria-label={`${titleOf(note)} 메뉴`} onClick={onOpen}><Icon name="more" size={18}/></button></div>
    {needsReveal && <Disclosure expanded={expanded} onToggle={() => setExpanded(!expanded)} targetId={id} label={`${titleOf(note)} 본문`}/>}
  </article>;
}

function descendants(id: string, notes: Note[]): Note[] {
  const children = notes.filter((note) => note.canvas.parent === id);
  return children.flatMap((child) => [child, ...(child.canvas.project ? descendants(child.id, notes) : [])]);
}
