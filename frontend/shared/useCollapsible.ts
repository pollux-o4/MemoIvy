import { type DependencyList, useLayoutEffect, useRef, useState } from 'react';

const measurements = new WeakMap<Element, () => void>();
let observer: ResizeObserver | undefined;

function observeContent(element: HTMLElement, measure: () => void) {
  observer ??= new ResizeObserver((entries) => {
    const changed = new Set(entries.map(({ target }) => measurements.get(target)));
    for (const read of changed) read?.();
  });
  const content = [...element.children];
  for (const child of content) {
    measurements.set(child, measure);
    observer.observe(child);
  }
  return () => {
    for (const child of content) {
      observer?.unobserve(child);
      measurements.delete(child);
    }
  };
}

/**
 * 접힘 높이를 넘는 내용을 가리고 펼침을 토글한다. 접힘 행과 전체 콘텐츠의 높이를 CSS 변수로 넘긴다.
 */
export function useCollapsible<T extends HTMLElement>(collapsedHeight: (element: T) => number, deps: DependencyList) {
  const ref = useRef<T>(null);
  const [expanded, setExpanded] = useState(false);
  const [extra, setExtra] = useState(0);
  const needsReveal = extra > 1;
  // biome-ignore lint/correctness/useExhaustiveDependencies: 호출자가 전달한 내용 변화가 관찰 대상을 바꾸는 기준이다.
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) { setExtra(0); return; }
    let previous: [number, number] | undefined;
    const measure = () => {
      const collapsed = collapsedHeight(element);
      const full = element.scrollHeight;
      if (previous?.[0] === collapsed && previous[1] === full) return;
      previous = [collapsed, full];
      element.style.setProperty('--preview-collapsed-height', `${collapsed}px`);
      element.style.setProperty('--preview-full-height', `${full}px`);
      setExtra(Math.max(0, full - collapsed));
    };
    measure();
    // 움직이는 클리핑 영역 대신 행을 관찰해 높이 전환의 프레임마다 다시 재지 않는다.
    return observeContent(element, measure);
  }, [...deps]);
  const className = `collapsible${needsReveal && !expanded ? ' has-more' : ''}${expanded ? ' is-expanded' : ''}`;
  return { ref, expanded, needsReveal, extra, className, toggle: () => setExpanded((value) => !value) };
}
