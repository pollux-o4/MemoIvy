import { type DependencyList, useLayoutEffect, useRef, useState } from 'react';

/**
 * 접힘 높이를 넘는 내용을 가리고(`collapsible`·`has-more`) 펼침을 토글한다. 목록형·캔버스 카드가 같이 쓴다.
 * `collapsedHeight`는 접힌 상태에서 보일 높이(px)를 요소에서 잰다. 짤 높이는 CSS 변수로만 넘긴다.
 */
export function useCollapsible<T extends HTMLElement>(collapsedHeight: (element: T) => number, deps: DependencyList) {
  const ref = useRef<T>(null);
  const [expanded, setExpanded] = useState(false);
  const [extra, setExtra] = useState(0);
  const needsReveal = extra > 1;
  // biome-ignore lint/correctness/useExhaustiveDependencies: 호출자가 준 내용 값과 펼침이 바뀔 때만 다시 잰다(값은 DOM에서 읽는다).
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) { setExtra(0); return; }
    const measure = () => {
      const collapsed = collapsedHeight(element);
      const full = element.scrollHeight;
      element.style.setProperty('--preview-collapsed-height', `${collapsed}px`);
      element.style.setProperty('--preview-full-height', `${full}px`);
      setExtra(Math.max(0, full - collapsed));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [...deps, expanded]);
  const className = `collapsible${needsReveal && !expanded ? ' has-more' : ''}${expanded ? ' is-expanded' : ''}`;
  /** extra: 펼치면 늘어나는 높이(px). 감싼 표면이 같은 모션으로 자랄 때 쓴다. */
  return { ref, expanded, needsReveal, extra, className, toggle: () => setExpanded((value) => !value) };
}
