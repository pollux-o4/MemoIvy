import { useEffect, useRef, useState } from 'react';

export type SaveLabel = '저장 중…' | '저장됨';
/** 입력·저장이 이만큼 멈춘 뒤에만 '저장됨'으로 바꾼다. 타이핑 중 자동저장마다 글자가 바뀌지 않게 한다. */
const SETTLE_MS = 1200;

/** GTK 저장 표시와 같은 두 상태. */
export function useSaveLabel(pending: boolean): SaveLabel {
  const [label, setLabel] = useState<SaveLabel>(pending ? '저장 중…' : '저장됨');
  const lastPendingAt = useRef(pending ? performance.now() : 0);
  if (pending) lastPendingAt.current = performance.now();
  useEffect(() => {
    if (pending) { setLabel('저장 중…'); return; }
    const wait = Math.max(0, SETTLE_MS - (performance.now() - lastPendingAt.current));
    const timer = window.setTimeout(() => setLabel('저장됨'), wait);
    return () => window.clearTimeout(timer);
  }, [pending]);
  return label;
}
