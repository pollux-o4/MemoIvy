import { useMemo } from 'react';
import type { Note } from '../../shared/engine_contract';
import { childCountLabel } from '../../shared/hierarchy';
import { Icon } from '../../shared/Icon';
import { progressLabel } from '../../shared/progress';
import { useProgress } from '../../shared/ProgressProvider';
import { PreviewFrame, usePreviewDisclosure } from '../../shared/markdown/preview/PreviewFrame';
import type { MemoStore } from '../../shared/useMemoivy';
import { libraryTitle } from './library_model';
import { NoteMenu } from './NoteMenu';
import { TaskCountLabel, TaskProgress } from '../../shared/TaskProgress';
import { markdownPreview } from '../../shared/markdown/preview/model';
import { PreviewList } from '../../shared/markdown/preview/PreviewList';
import { savePreviewTask } from '../../shared/markdown/preview/savePreviewTask';

export function NoteCard({ note, path, store, onOpenCanvas, nested = false, onOpen }: { note: Note; path: string[]; store: MemoStore; onOpenCanvas?: (id?: string) => void; nested?: boolean; onOpen: () => void }) {
  const progress = useProgress();
  const { rows } = useMemo(() => markdownPreview(note.text), [note.text]);
  const collapsedRows = nested ? 1 : 2;
  const task = progress.of(note);
  const taskText = progressLabel(note, progress);
  const label = libraryTitle(note);
  const id = `preview-${note.id}`;
  const preview = usePreviewDisclosure(rows, collapsedRows);

  return <article className={`note-card card ${note.color}${note.canvas.project ? ' project' : ''}${nested ? ' nested' : ''}`}>
    <div className="card-row"><div className="card-stripe"/><div className="card-content">
      {path.length > 0 && <div className="card-location">{path.join(' / ')}</div>}
      <div className="card-title-row">
        <button type="button" className="card-open" onClick={onOpen}>
          {note.canvas.project && <span className="project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>}
          <strong className="card-title">{label}</strong>
        </button>
        {task.total > 0 && <TaskCountLabel progress={task} title={taskText.title} className="task-chip">{taskText.short}</TaskCountLabel>}
      </div>
      {note.canvas.project && <span className="preview">{childCountLabel(progress.children(note.id).length)}</span>}
      {rows.length > 0 && <PreviewFrame label={label} targetId={id} preview={preview}>
        <PreviewList ref={preview.ref} className={`preview ${preview.className}`} id={id} rows={rows}
          visibleRows={preview.expanded ? rows.length : collapsedRows} onToggleTask={(line) => savePreviewTask(store, note, line)}/>
      </PreviewFrame>}
      {!note.canvas.project && !note.text.trim() && <span className="preview">열어서 내용을 적어 보세요</span>}
      {note.canvas.project && task.total > 0 && <TaskProgress className="project-progress" label={label} progress={task}/>}
    </div>{note.canvas.project && onOpenCanvas && <button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${label} 캔버스 열기`} onClick={() => onOpenCanvas(note.id)}><Icon name="enter" size={17}/></button>}<NoteMenu id={note.id} title={label} store={store}/></div>
  </article>;
}
