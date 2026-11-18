import { useCompletionMoment } from './useCompletionMoment';

export type TaskCount = { done: number; total: number };

/**
 * 할 일 진행 막대. 목록형·캔버스가 함께 쓰고 겉모양은 className이 정한다.
 * 채움은 너비 대신 scaleX로 바꿔 motion.css의 전이가 레이아웃 없이 부드럽게 차오른다.
 * 모두 끝나는 순간에는 data-moment로 한 번만 빛이 지나간다(motion.css task-complete).
 */
export function TaskProgress({ label, tasks, className }: { label: string; tasks: TaskCount; className: string }) {
  const moment = useCompletionMoment(tasks);
  return <span className={`task-progress ${className}`} data-moment={moment || undefined} role="progressbar" aria-label={`${label} 할 일 진행률`} aria-valuemin={0} aria-valuemax={tasks.total} aria-valuenow={tasks.done}>
    <i style={{ transform: `scaleX(${tasks.total ? tasks.done / tasks.total : 0})` }}/>
  </span>;
}

/** 할 일 개수 표시. 완료 순간에 진행 막대와 함께 살짝 튄다. */
export function TaskCountLabel({ tasks, className, children }: { tasks: TaskCount; className?: string; children: React.ReactNode }) {
  const moment = useCompletionMoment(tasks);
  return <span className={`task-count${className ? ` ${className}` : ''}`} data-moment={moment || undefined} title={`할 일 ${tasks.total}개 중 ${tasks.done}개 완료`}>{children}</span>;
}
