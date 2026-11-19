import { useMemo, useRef } from 'react';

// biome-ignore lint/suspicious/noExplicitAny: 동작 묶음의 인자 형태는 호출하는 쪽 타입이 정한다.
type Handlers = Record<string, (...args: any[]) => unknown>;

/**
 * 매 렌더 새로 만든 동작 묶음을 참조가 고정된 묶음으로 바꾼다. 호출 시점에는 항상 최신 동작을 부른다.
 * memo 부품(카드·인스펙터)에 넘겨 끌기 중 불필요한 다시 그리기를 막는다. 키 목록은 처음 렌더 기준으로 고정한다.
 */
export function useStableHandlers<T extends Handlers>(handlers: T): T {
  const latest = useRef(handlers);
  latest.current = handlers;
  // biome-ignore lint/correctness/useExhaustiveDependencies: 키는 첫 렌더에 고정하고 호출은 latest로 위임한다.
  return useMemo(() => Object.fromEntries(Object.keys(handlers).map((key) => [key, (...args: unknown[]) => latest.current[key](...args)])) as T, []);
}
