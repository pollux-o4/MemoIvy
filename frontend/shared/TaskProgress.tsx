import type { ReactNode } from 'react';
import type { Progress } from './progress';
import { useCompletionMoment } from './useCompletionMoment';
import './task_progress.css';

/**
 * 진행 막대. 목록형·캔버스가 함께 쓰고 겉모양은 className이 정한다.
 * 채움은 너비 대신 scaleX로 바꿔 motion.css의 전이가 레이아웃 없이 부드럽게 차오른다.
 * 모두 끝나는 순간에는 data-moment로 한 번만 빛이 지나간다(task_progress.css).
 */
export function TaskProgress({ label, progress, className }: { label: string; progress: Progress; className: string }) {
  const moment = useCompletionMoment(progress);
  return <span className={`task-progress ${className}`} data-moment={moment || undefined} role="progressbar" aria-label={`${label} 진행률`} aria-valuemin={0} aria-valuemax={progress.total} aria-valuenow={progress.done}>
    <i style={{ transform: `scaleX(${progress.total ? progress.done / progress.total : 0})` }}/>
  </span>;
}

/** 진행 개수 표시. 완료 순간에 진행 막대와 함께 살짝 튄다. */
export function TaskCountLabel({ progress, title, className, children }: { progress: Progress; title: string; className?: string; children: ReactNode }) {
  const moment = useCompletionMoment(progress);
  const complete = progress.total > 0 && progress.done === progress.total;
  return <span className={`task-count${complete ? ' complete' : ''}${className ? ` ${className}` : ''}`} data-moment={moment || undefined} title={title}>{children}</span>;
}
