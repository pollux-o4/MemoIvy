export type TaskCount = { done: number; total: number };

/**
 * 할 일 진행 막대. 목록형·캔버스가 함께 쓰고 겉모양은 className이 정한다.
 * 채움은 너비 대신 scaleX로 바꿔 motion.css의 전이가 레이아웃 없이 부드럽게 차오른다.
 */
export function TaskProgress({ label, tasks, className }: { label: string; tasks: TaskCount; className: string }) {
  return <span className={`task-progress ${className}`} role="progressbar" aria-label={`${label} 할 일 진행률`} aria-valuemin={0} aria-valuemax={tasks.total} aria-valuenow={tasks.done}>
    <i style={{ transform: `scaleX(${tasks.total ? tasks.done / tasks.total : 0})` }}/>
  </span>;
}
