import { useMemo, useState } from 'react';
import type { Note } from '../../shared/engine_contract';
import { Disclosure } from '../../shared/Disclosure';
import { checklist, noteColor } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';
import { descendantNotes, libraryTitle, orderedLibraryChildren } from './library_model';
import { NoteCard } from './NoteCard';
import { ProjectHeading } from './ProjectHeading';
import { ProjectStack } from './ProjectStack';

export function ProjectSection({ project, notes, store, expandedIds, onToggle, onOpenNote, onOpenCanvas, nested = false }: { project: Note; notes: Note[]; store: MemoStore; expandedIds: string[]; onToggle: (id: string) => void; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void; nested?: boolean }) {
  const expanded = expandedIds.includes(project.id);
  const [everExpanded, setEverExpanded] = useState(expanded);
  const children = useMemo(() => orderedLibraryChildren(project.id, notes), [project.id, notes]);
  const tasks = checklist([project.text, ...descendantNotes(project.id, notes).map((note) => note.text)].join('\n'));
  const label = libraryTitle(project);
  const regionId = `section-${project.id}`;
  const toggle = () => { setEverExpanded(true); onToggle(project.id); };
  return <ProjectStack childCount={children.length} expanded={expanded} nested={nested} accent={noteColor(project.color).accent}>
    <article className={`project-section${nested ? ' nested' : ''}`}>
    <ProjectHeading id={project.id} label={label} childCount={children.length} tasks={tasks} store={store} onOpenNote={onOpenNote} onOpenCanvas={onOpenCanvas}/>
    {children.length > 0 && <><div className={`project-contents${expanded ? ' expanded' : ''}`} id={regionId} aria-hidden={!expanded} inert={!expanded}><div className="project-content-inner">{everExpanded && children.map((child) => child.canvas.project ? <ProjectSection key={child.id} project={child} notes={notes} store={store} expandedIds={expandedIds} onToggle={onToggle} onOpenNote={onOpenNote} onOpenCanvas={onOpenCanvas} nested/> : <NoteCard key={child.id} note={child} path={[label]} store={store} nested onOpen={() => onOpenNote(child.id)}/>)}</div></div><Disclosure expanded={expanded} onToggle={toggle} targetId={regionId} label={label}/></>}
  </article></ProjectStack>;
}
