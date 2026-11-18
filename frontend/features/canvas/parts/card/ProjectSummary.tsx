import { type TaskCount, TaskProgress } from '../../../../shared/TaskProgress';

/** 프로젝트 카드 아래쪽: 하위 메모 수·할 일 합계와 진행 막대. */
export function ProjectSummary({ label, count, tasks }: { label: string; count: number; tasks: TaskCount }) {
  return <>
    <div className="canvas-project-meta"><small><b>메모 {count}</b></small>{tasks.total > 0 && <small>할 일 {tasks.done}/{tasks.total}</small>}</div>
    {tasks.total > 0 && <TaskProgress className="canvas-project-progress" label={label} tasks={tasks}/>}
  </>;
}
