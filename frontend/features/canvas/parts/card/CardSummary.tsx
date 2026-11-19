import { childCountLabel } from '../../../../shared/hierarchy';
import type { Progress, progressLabel } from '../../../../shared/progress';
import { TaskCountLabel, TaskProgress } from '../../../../shared/TaskProgress';

export function CardSummary({ label, childCount, progress, text }: { label: string; childCount?: number; progress: Progress; text: ReturnType<typeof progressLabel> }) {
  const project = childCount !== undefined;
  return <>
    <div className={project ? 'canvas-project-meta' : 'canvas-card-meta'}>
      {project && <small><b>{childCountLabel(childCount)}</b></small>}
      {progress.total > 0 && <small><TaskCountLabel progress={progress} title={text.title}>{project ? '완료' : '할 일'} {text.short}</TaskCountLabel></small>}
    </div>
    {progress.total > 0 && <TaskProgress className="canvas-card-progress" label={label} progress={progress}/>}
  </>;
}
