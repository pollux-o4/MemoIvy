/**
 * 초안 저장 시점 정책(Strategy). 입력마다 schedule을 부르면 정책이 정한 때에 save가 한 번 불린다.
 * 같은 규칙을 편집기·인스펙터가 공유한다(SSoT).
 */
export interface SavePolicy {
  /** 마지막 입력 뒤 저장까지 기다리는 시간(ms). */
  idle: number;
  /** 계속 입력해도 첫 입력 뒤 이 시간 안에는 한 번 저장한다(ms). */
  maxWait: number;
}

/** 목록형 편집기와 같은 실시간 반영 속도. */
export const LIVE_SAVE: SavePolicy = { idle: 300, maxWait: 1000 };

export interface Autosave<T> {
  schedule: (value: T) => void;
  /** 기다리던 저장을 지금 실행한다. 기다리던 값이 없으면 아무것도 안 한다. */
  flush: () => void;
  cancel: () => void;
  pending: () => boolean;
}

export function createAutosave<T>(policy: SavePolicy, save: (value: T) => void, timers: Pick<typeof globalThis, 'setTimeout' | 'clearTimeout'> = globalThis): Autosave<T> {
  let idle: ReturnType<typeof setTimeout> | undefined;
  let started: ReturnType<typeof setTimeout> | undefined;
  let waiting: { value: T } | undefined;
  const cancel = () => {
    timers.clearTimeout(idle); timers.clearTimeout(started);
    idle = started = undefined; waiting = undefined;
  };
  const flush = () => {
    if (!waiting) return;
    const { value } = waiting;
    cancel();
    save(value);
  };
  return {
    schedule: (value) => {
      waiting = { value };
      timers.clearTimeout(idle);
      idle = timers.setTimeout(flush, policy.idle);
      if (started === undefined) started = timers.setTimeout(flush, policy.maxWait);
    },
    flush, cancel, pending: () => waiting !== undefined,
  };
}
