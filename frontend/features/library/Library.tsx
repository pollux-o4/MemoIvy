import { useEffect, useMemo, useRef, useState } from 'react';
import { ProgressProvider } from '../../shared/ProgressProvider';
import type { KeyboardEvent } from 'react';
import { WindowHeader } from '../../shared/WindowHeader';
import { Icon } from '../../shared/Icon';
import type { MemoStore } from '../../shared/useMemoivy';
import { CreationMenu } from '../../shared/menu/CreationMenu';
import { NoteCard } from './NoteCard';
import { ProjectSection } from './ProjectSection';
import { libraryPath, orderedLibraryChildren, searchLibraryNotes } from './library_model';
import libraryEmptyIllustration from '../../../src/presentation/icons/illustrations/library-empty.svg?url';
import './library.css';


export function Library({ store, onOpenNote, onOpenCanvas, onCreate, onOpenSettings }: { store: MemoStore; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void; onCreate: (project: boolean) => void; onOpenSettings: () => void }) {
  const [query, setQuery] = useState('');
  const [mode, setMode] = useState<'projects' | 'recent'>('projects');
  const [expanded, setExpanded] = useState<string[]>(() => (store.snapshot?.preferences?.expandedProjects as string[] | undefined) ?? []);
  const expandedRef = useRef(expanded);
  const projects = store.notes.filter((note) => note.canvas.project);
  const matches = useMemo(() => searchLibraryNotes(store.notes, query), [store.notes, query]);
  const searching = query.length > 0;
  const displayed = matches;
  const topLevel = useMemo(() => orderedLibraryChildren(null, store.notes), [store.notes]);

  useEffect(() => {
    const saved = (store.snapshot?.preferences?.expandedProjects as string[] | undefined) ?? [];
    expandedRef.current = saved;
    setExpanded(saved);
  }, [store.snapshot?.preferences?.expandedProjects]);

  const toggleExpanded = (id: string) => {
    const next = expandedRef.current.includes(id) ? expandedRef.current.filter((item) => item !== id) : [...expandedRef.current, id];
    expandedRef.current = next;
    setExpanded(next);
    void store.request('preference', { key: 'expandedProjects', value: next }).catch(() => undefined);
  };
  const create = (project: boolean) => onCreate(project);
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const moveTab = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === 'Home' ? 'projects' : event.key === 'End' ? 'recent' : event.key === 'ArrowRight' ? 'recent' : 'projects';
    setMode(next);
    tabRefs.current[next === 'projects' ? 0 : 1]?.focus();
  };

  return <ProgressProvider notes={store.notes}><div className="library-screen">
    <WindowHeader title="MemoIvy" bridge={store.bridge} onClose={() => store.bridge.windowAction('close')} actions={<><button className="icon-button canvas-launch" title="캔버스 열기" aria-label="캔버스 열기" onClick={() => onOpenCanvas()}><Icon name="canvas"/></button><button className="icon-button settings-launch" aria-label="설정" onClick={onOpenSettings}><Icon name="settings"/></button><CreationMenu onCreate={create} disabled={store.saving}/></>}/>
    <main className="library-main">
      <label className="search-field"><Icon name="search" size={17}/>{/* biome-ignore lint/a11y/noAutofocus: 목록 창은 검색이 주 진입점이라 열자마자 입력을 받는다. */}<input id="library-search" autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="메모 검색" aria-label="메모 검색" title="메모 검색 · Ctrl+F"/>{query && <button aria-label="검색 지우기" onClick={() => setQuery('')}><Icon name="close" size={15}/></button>}</label>
      {(searching || store.notes.length > 0) && <div className="count-bar"><span className="count">{searching ? `검색 결과 ${displayed.length}개` : `메모 ${store.notes.length}개${projects.length > 0 ? ` · 프로젝트 ${projects.length}개` : ''}`}</span>{!searching && projects.length > 0 && <div className="segmented" role="tablist" aria-label="목록 보기"><button ref={(element) => { tabRefs.current[0] = element; }} role="tab" tabIndex={mode === 'projects' ? 0 : -1} aria-selected={mode === 'projects'} className={mode === 'projects' ? 'selected' : ''} onClick={() => setMode('projects')} onKeyDown={moveTab}>프로젝트별</button><button ref={(element) => { tabRefs.current[1] = element; }} role="tab" tabIndex={mode === 'recent' ? 0 : -1} aria-selected={mode === 'recent'} className={mode === 'recent' ? 'selected' : ''} onClick={() => setMode('recent')} onKeyDown={moveTab}>최근</button></div>}</div>}
      {!store.notes.length && !searching ? <section className="note-list" aria-label="메모 목록"><div className="empty-state is-first"><img src={libraryEmptyIllustration} width={96} height={80} alt=""/><strong className="card-title">첫 메모를 남겨 보세요</strong><p>+ New에서 메모를 만들어 생각을 적어 보세요.</p></div></section> : searching || mode === 'recent' ? <section className="note-list" aria-label="메모 목록">{displayed.length ? displayed.map((note) => <NoteCard key={note.id} note={note} path={searching ? libraryPath(note, store.notes) : []} store={store} onOpenCanvas={onOpenCanvas} onOpen={() => onOpenNote(note.id)}/>) : <div className="empty-state"><strong className="card-title">일치하는 메모가 없어요</strong><p>다른 검색어를 입력해 보세요.</p></div>}</section> : <section className="note-list" aria-label="메모와 프로젝트">{topLevel.map((note) => note.canvas.project ? <ProjectSection key={note.id} project={note} notes={store.notes} store={store} expandedIds={expanded} onToggle={toggleExpanded} onOpenNote={onOpenNote} onOpenCanvas={onOpenCanvas}/> : <NoteCard key={note.id} note={note} path={[]} store={store} onOpen={() => onOpenNote(note.id)}/>)}</section>}
    </main>
  </div></ProgressProvider>;
}
