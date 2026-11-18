import { type DependencyList, useLayoutEffect, useRef, useState } from 'react';

/**
 * 접힘 높이를 넘는 내용을 가리고(`collapsible`·`has-more`) 펼침을 토글한다. 목록형·캔버스 카드가 같이 쓴다.
 * `collapsedHeight`는 접힌 상태에서 보일 높이(px)를 요소에서 잰다. 짤 높이는 CSS 변수로만 넘긴다.
 */
export function useCollapsible<T extends HTMLElement>(collapsedHeight: (element: T) => number, deps: DependencyList) {
  const ref = useRef<T>(null);
  const [expanded, setExpanded] = useState(false);
  const [needsReveal, setNeedsReveal] = useState(false);
  // biome-ignore lint/correctness/useExhaustiveDependencies: 호출자가 준 내용 값과 펼침이 바뀔 때만 다시 잰다(값은 DOM에서 읽는다).
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) { setNeedsReveal(false); return; }
    const measure = () => {
      const collapsed = collapsedHeight(element);
      const full = element.scrollHeight;
      element.style.setProperty('--preview-collapsed-height', `${collapsed}px`);
      element.style.setProperty('--preview-full-height', `${full}px`);
      setNeedsReveal(full > collapsed + 1);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [...deps, expanded]);
  const className = `collapsible${needsReveal && !expanded ? ' has-more' : ''}${expanded ? ' is-expanded' : ''}`;
  return { ref, expanded, needsReveal, className, toggle: () => setExpanded((value) => !value) };
}
