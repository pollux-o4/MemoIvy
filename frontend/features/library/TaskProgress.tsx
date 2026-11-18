export type TaskCount = { done: number; total: number };

export function TaskProgress({ label, tasks }: { label: string; tasks: TaskCount }) {
  return <span className="project-progress" role="progressbar" aria-label={`${label} 할 일 진행률`} aria-valuemin={0} aria-valuemax={tasks.total} aria-valuenow={tasks.done}><i style={{ width: `${tasks.done / tasks.total * 100}%` }}/></span>;
}
