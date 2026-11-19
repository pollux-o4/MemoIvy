import { childCountLabel } from '../../../../shared/hierarchy';
import type { Progress } from '../../../../shared/progress';
import { TaskCountLabel, TaskProgress } from '../../../../shared/TaskProgress';

/** 프로젝트 카드 아래쪽: 하위 메모 수, 하위 항목 단위 진행과 막대. */
export function ProjectSummary({ label, count, progress, text }: { label: string; count: number; progress: Progress; text: { short: string; title: string } }) {
  return <>
    <div className="canvas-project-meta"><small><b>{childCountLabel(count)}</b></small>{progress.total > 0 && <small><TaskCountLabel progress={progress} title={text.title}>완료 {text.short}</TaskCountLabel></small>}</div>
    {progress.total > 0 && <TaskProgress className="canvas-project-progress" label={label} progress={progress}/>}
  </>;
}
