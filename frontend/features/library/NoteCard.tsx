import type { Note } from '../../shared/engine_contract';
import { Disclosure } from '../../shared/Disclosure';
import { childCountLabel } from '../../shared/hierarchy';
import { Icon } from '../../shared/Icon';
import { progressLabel } from '../../shared/progress';
import { useProgress } from '../../shared/ProgressProvider';
import { useCollapsible } from '../../shared/useCollapsible';
import type { MemoStore } from '../../shared/useMemoivy';
import { libraryBody, libraryTitle } from './library_model';
import { NoteMenu } from './NoteMenu';
import { TaskCountLabel, TaskProgress } from '../../shared/TaskProgress';

export function NoteCard({ note, path, store, onOpenCanvas, nested = false, onOpen }: { note: Note; path: string[]; store: MemoStore; onOpenCanvas?: (id?: string) => void; nested?: boolean; onOpen: () => void }) {
  const progress = useProgress();
  const summary = note.canvas.project ? childCountLabel(progress.children(note.id).length) : libraryBody(note.text) || (note.text.trim() ? '' : '열어서 내용을 적어 보세요');
  const task = progress.of(note);
  const taskText = progressLabel(note, progress);
  const label = libraryTitle(note);
  const id = `preview-${note.id}`;
  const preview = useCollapsible<HTMLDivElement>((element) => parseFloat(getComputedStyle(element).lineHeight) * (nested ? 1 : 2), [summary, nested]);

  return <article className={`note-card card ${note.color}${note.canvas.project ? ' project' : ''}${nested ? ' nested' : ''}`}>
    <div className="card-row"><div className="card-stripe"/><button className="card-open" onClick={onOpen}><div className="card-content">
      {path.length > 0 && <div className="card-location">{path.join(' / ')}</div>}
      <div className="card-title-row">{note.canvas.project && <span className="project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>}<strong className="card-title">{label}</strong>{task.total > 0 && <TaskCountLabel progress={task} title={taskText.title} className="task-chip">{taskText.short}</TaskCountLabel>}</div>
      {summary && <div ref={preview.ref} className={`preview ${preview.className}${nested ? ' nested-preview' : ''}`} id={id} aria-hidden={false}>{summary}</div>}
      {note.canvas.project && task.total > 0 && <TaskProgress className="project-progress" label={label} progress={task}/>}
    </div></button>{note.canvas.project && onOpenCanvas && <button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${label} 캔버스 열기`} onClick={() => onOpenCanvas(note.id)}><Icon name="enter" size={17}/></button>}<NoteMenu id={note.id} title={label} store={store}/></div>
    {preview.needsReveal && <Disclosure expanded={preview.expanded} onToggle={preview.toggle} targetId={id} label={`${label} 본문`}/>}
  </article>;
}
