import { type RefObject, useEffect, useEffectEvent, useRef } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { bounds, fitting, type Rect } from '../../model/motion';
import { useCanvasMotion } from './useCanvasMotion';
import { useCanvasWheel } from './useCanvasWheel';
import { useRevealSelection } from './useRevealSelection';

/**
 * 캔버스 카메라 파사드: 움직임·휠·선택 드러내기·전체 보기·100% 보기와 범위 전환 시 맞춤, 첫 연결 신호를 한곳에 묶는다.
 * 화면 조립(Canvas)은 이 반환값만 쓴다.
 */
export function useCanvasCamera(o: {
  host: RefObject<HTMLDivElement | null>; notes: Note[]; noteById: Map<string, Note>; scope: string | null;
  cardRects: Rect[]; selected: string[]; selectedRects: Rect[]; idle: boolean;
}) {
  const motion = useCanvasMotion(o.host, o.cardRects, o.selectedRects);
  const { camera, animateTo, triggerLink } = motion;
  useCanvasWheel(o.host, motion);
  const single = o.selected.length === 1 && o.selectedRects.length === 1 ? { id: o.selected[0], rect: o.selectedRects[0] } : undefined;
  const reveal = useRevealSelection(o.host, single, camera, animateTo, o.idle);
  const viewport = (): [number, number] | null => { const board = o.host.current?.getBoundingClientRect(); return board ? [board.width, board.height] : null; };
  const fitView = (onlySelection = false) => {
    const size = viewport(); const target = bounds(onlySelection ? o.selectedRects : o.cardRects);
    if (size && target) animateTo(fitting(target, size));
  };
  const resetZoom = () => {
    const size = viewport(); if (!size) return;
    const [width, height] = size;
    animateTo({ x: camera.x + width / camera.zoom / 2 - width / 2, y: camera.y + height / camera.zoom / 2 - height / 2, zoom: 1 });
  };

  // 범위(프로젝트)가 바뀌면 새 범위 전체를 맞춰 보여 준다.
  const fitScope = useEffectEvent(() => fitView());
  const previousScope = useRef(o.scope);
  useEffect(() => {
    if (previousScope.current === o.scope) return;
    previousScope.current = o.scope;
    const frame = requestAnimationFrame(fitScope);
    return () => cancelAnimationFrame(frame);
  }, [o.scope]);
  // 처음 메모가 보일 때 기존 연결선에 신호를 한 번 흘려 방향을 알린다.
  const seededLinks = useRef(false);
  useEffect(() => {
    if (seededLinks.current || !o.notes.length) return;
    seededLinks.current = true;
    for (const note of o.notes) for (const target of note.canvas.next) if (o.noteById.has(target)) triggerLink(`${note.id}:${target}`, false);
  }, [o.notes, o.noteById, triggerLink]);

  return { motion, camera, triggerLink, reveal, fitView, resetZoom };
}
