import { useEffect, useRef } from 'react';

/** 면이 바뀐 뒤 포커스 위치: 들어가면 새 면의 첫 동작, 돌아오면 방금 나온 묶음 버튼. 첫 표시에는 포커스를 옮기지 않는다. */
export function useFaceFocus<Face extends string>(face: Face, root: Face) {
  const bar = useRef<HTMLDivElement>(null);
  const previous = useRef<Face | null>(null);
  useEffect(() => {
    const from = previous.current;
    if (from === null) return;
    const target = face === root
      ? bar.current?.querySelector<HTMLElement>(`[data-group="${from}"]`)
      : bar.current?.querySelector<HTMLElement>('[data-action]:not(:disabled)');
    target?.focus({ focusVisible: false } as FocusOptions);
  }, [face, root]);
  /** 면을 바꾸기 직전에 부른다. 이전 면이 포커스 복귀와 진입 방향의 기준이다. */
  const leaving = (current: Face) => { previous.current = current; };
  return { bar, leaving, returned: previous.current !== null };
}
