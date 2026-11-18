import { useLayoutEffect, useRef, useState } from 'react';
import type { Note } from '../../shared/engine_contract';
import { Disclosure } from '../../shared/Disclosure';
import { Icon } from '../../shared/Icon';
import { checklist } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';
import { libraryBody, libraryTitle } from './library_model';
import { NoteMenu } from './NoteMenu';
import { TaskProgress } from './TaskProgress';

export function NoteCard({ note, path, store, onOpenCanvas, nested = false, onOpen }: { note: Note; path: string[]; store: MemoStore; onOpenCanvas?: (id?: string) => void; nested?: boolean; onOpen: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const [needsReveal, setNeedsReveal] = useState(false);
  const previewRef = useRef<HTMLDivElement>(null);
  const childCount = store.notes.filter((child) => child.canvas.parent === note.id).length;
  const summary = note.canvas.project ? `메모 ${childCount}개` : libraryBody(note.text) || (note.text.trim() ? '' : '열어서 내용을 적어 보세요');
  const task = checklist(note.text);
  const label = libraryTitle(note);
  const id = `preview-${note.id}`;
  useLayoutEffect(() => {
    const element = previewRef.current;
    if (!element) { setNeedsReveal(false); return; }
    const measure = () => {
      const collapsedHeight = parseFloat(getComputedStyle(element).lineHeight) * (nested ? 1 : 2);
      const fullHeight = element.scrollHeight;
      element.style.setProperty('--preview-collapsed-height', `${collapsedHeight}px`);
      element.style.setProperty('--preview-full-height', `${fullHeight}px`);
      setNeedsReveal(fullHeight > collapsedHeight + 1);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [summary, expanded, nested]);

  return <article className={`note-card card ${note.color}${note.canvas.project ? ' project' : ''}${nested ? ' nested' : ''}`}>
    <div className="card-row"><div className="card-stripe"/><button className="card-open" onClick={onOpen}><div className="card-content">
      {path.length > 0 && <div className="card-location">{path.join(' / ')}</div>}
      <div className="card-title-row">{note.canvas.project && <span className="project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>}<strong className="card-title">{label}</strong>{task.total > 0 && <span className={`task-chip${task.done === task.total ? ' complete' : ''}`} title={`할 일 ${task.total}개 중 ${task.done}개 완료`}>{task.done}/{task.total}</span>}</div>
      {summary && <div ref={previewRef} className={`preview${nested ? ' nested-preview' : ''}${needsReveal && !expanded ? ' has-more' : ''}${expanded ? ' is-expanded' : ''}`} id={id} aria-hidden={false}>{summary}</div>}
      {note.canvas.project && task.total > 0 && <TaskProgress label={label} tasks={task}/>}
    </div></button>{note.canvas.project && onOpenCanvas && <button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${label} 캔버스 열기`} onClick={() => onOpenCanvas(note.id)}><Icon name="enter" size={17}/></button>}<NoteMenu id={note.id} title={label} store={store}/></div>
    {needsReveal && <Disclosure expanded={expanded} onToggle={() => setExpanded((value) => !value)} targetId={id} label={`${label} 본문`}/>}
  </article>;
}
