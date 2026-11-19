import { useCallback, useEffect, useState } from 'react';
import { prefersReducedMotion } from '../appearance';

export type Presence = 'closed' | 'open' | 'closing';

/** 퇴장 모션 동안 DOM을 유지하는 열림 상태. 모션 줄이기면 즉시 닫는다. */
export function usePresence(exitMs: number, initial: Presence = 'closed') {
  const [state, setState] = useState<Presence>(initial);
  const show = useCallback(() => setState('open'), []);
  const hide = useCallback(() => setState((current) => current === 'closed' ? current : prefersReducedMotion() ? 'closed' : 'closing'), []);
  const dismiss = useCallback(() => setState('closed'), []);
  useEffect(() => {
    if (state !== 'closing') return;
    const timer = window.setTimeout(() => setState('closed'), exitMs);
    return () => window.clearTimeout(timer);
  }, [state, exitMs]);
  return { state, open: state === 'open', mounted: state !== 'closed', show, hide, dismiss };
}
