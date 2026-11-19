import { type RefObject, useEffect, useRef } from 'react';
import { wheelIntent } from '../../model/wheel';

type Camera = { panBy: (delta: [number, number], smooth: boolean) => void; zoomAtScreen: (point: [number, number], factor: number) => void };

/** React onWheel은 passive라 Ctrl+휠의 페이지 확대·스크롤을 막지 못해 직접 등록한다. */
export function useCanvasWheel(host: RefObject<HTMLDivElement | null>, camera: Camera) {
  const latest = useRef(camera);
  latest.current = camera;
  useEffect(() => {
    const board = host.current;
    if (!board) return;
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey && !event.shiftKey && Math.abs(event.deltaY) > Math.abs(event.deltaX) && event.target instanceof Element) {
        const preview = event.target.closest<HTMLElement>('.canvas-checklist-preview.is-expanded');
        // 본문 스크롤과 카메라 이동을 같은 휠 제스처에서 섞지 않는다.
        if (preview && preview.scrollHeight > preview.clientHeight + 1) return;
      }
      event.preventDefault();
      const intent = wheelIntent(event, window.devicePixelRatio || 1);
      if (intent.kind === 'zoom') {
        const rect = board.getBoundingClientRect();
        latest.current.zoomAtScreen([event.clientX - rect.left, event.clientY - rect.top], intent.factor);
      } else latest.current.panBy(intent.delta, intent.smooth);
    };
    board.addEventListener('wheel', onWheel, { passive: false });
    return () => board.removeEventListener('wheel', onWheel);
  }, [host]);
}
