import { useEffect, useRef, useState } from 'react';

/** 완료 순간 효과가 머무는 시간. task_progress.css의 task-swell 애니메이션 길이와 같다. */
const MOMENT_MS = 900;

/**
 * 할 일이 모두 끝나는 "순간"만 알린다. 처음부터 100%였거나 다시 열었을 때는 알리지 않는다.
 * 진행 막대와 개수 표시가 같은 순간을 공유해 효과가 한 번에 맞물린다.
 */
export function useCompletionMoment({ done, total }: { done: number; total: number }): boolean {
  const complete = total > 0 && done === total;
  const previous = useRef(complete);
  const [moment, setMoment] = useState(false);
  useEffect(() => {
    const reached = complete && !previous.current;
    previous.current = complete;
    if (!reached) { if (!complete) setMoment(false); return; }
    setMoment(true);
    const timer = window.setTimeout(() => setMoment(false), MOMENT_MS);
    return () => window.clearTimeout(timer);
  }, [complete]);
  return moment;
}
