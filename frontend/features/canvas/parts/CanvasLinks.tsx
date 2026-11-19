import { memo, useMemo, useRef } from 'react';
import type { Note } from '../../../shared/engine_contract';
import type { CanvasPoint } from '../model/align';
import { CARD_HEIGHT, CARD_WIDTH } from '../model/align';
import { layoutAnchors } from '../model/links/anchors';
import { PATH_STRATEGIES } from '../model/links/paths';
import { type Edge, type EdgeAnchors, edgeKey, type LinkStyle, type Rect, type Side } from '../model/links/types';

type LinkSample = { creating: boolean; pulsing: boolean; progress: number; travel: number; alpha: number };
const RESTING: LinkSample = { creating: false, pulsing: false, progress: 1, travel: 0, alpha: 1 };

/** 이 화면 단계 밖(프로젝트 안쪽)으로 이어진 연결을 그 단계에 보이는 대표 카드끼리의 점선으로 투영한다. */
function projectedEdges(notes: Note[], allNotes: Note[], scope: string | null): Edge[] {
  const noteIds = new Set(notes.map((note) => note.id));
  const byId = new Map(allNotes.map((note) => [note.id, note]));
  const representative = (id: string): string | undefined => {
    let note = byId.get(id);
    const seen = new Set<string>();
    while (note && note.canvas.parent !== scope && note.canvas.parent && !seen.has(note.id)) { seen.add(note.id); note = byId.get(note.canvas.parent); }
    return note?.canvas.parent === scope ? note.id : undefined;
  };
  const pairs = new Set<string>();
  for (const note of allNotes) for (const target of note.canvas.next) {
    if (noteIds.has(note.id) && noteIds.has(target)) continue;
    const from = representative(note.id); const to = representative(target);
    if (from && to && from !== to && !byId.get(from)?.canvas.next.includes(to)) pairs.add(`${from}:${to}`);
  }
  return [...pairs].map((pair) => { const [from, to] = pair.split(':'); return { from, to }; });
}

/**
 * 연결선 그리기: 연결점·경로는 model/links가 계산하고 여기서는 SVG로만 옮긴다.
 * focus가 있으면 그 가지 밖의 선을 흐리게 한다(가지 초점).
 */
export const CanvasLinks = memo(function CanvasLinks({ notes, allNotes, scope, positions, zoom, selected, sampleForLink, linkStyle = 'curve', focus }: {
  notes: Note[]; allNotes: Note[]; scope: string | null; positions: Record<string, CanvasPoint>; zoom: number; selected: string[];
  sampleForLink: (key: string) => LinkSample; linkStyle?: LinkStyle; focus?: ReadonlySet<string>;
}) {
  const direct = useMemo(() => {
    const ids = new Set(notes.map((note) => note.id));
    return notes.flatMap((note) => note.canvas.next.filter((target) => ids.has(target)).map((to) => ({ from: note.id, to })));
  }, [notes]);
  const projected = useMemo(() => projectedEdges(notes, allNotes, scope), [notes, allNotes, scope]);
  // shortcut: 카드 높이는 기본값으로 계산(미리보기 펼침 높이 미반영), 펼친 카드 연결점이 어긋나 보이면 카드 높이를 상위 상태로 올린다.
  const rects = useMemo(() => {
    const out: Record<string, Rect> = {};
    for (const [id, point] of Object.entries(positions)) out[id] = { x: point.x, y: point.y, w: CARD_WIDTH, h: CARD_HEIGHT };
    return out;
  }, [positions]);
  // 직전 프레임의 변을 기억해 경계 근처에서 연결점이 깜빡이지 않게 한다. 사라진 간선 기록은 매번 새 맵으로 버린다.
  const memory = useRef<Map<string, { from: Side; to: Side }>>(new Map());
  const anchors = useMemo(() => {
    const edges = [...direct, ...projected];
    const result = layoutAnchors(rects, edges, memory.current);
    memory.current = new Map(result.map((item) => [item.key, { from: item.from.side, to: item.to.side }]));
    return result;
  }, [direct, projected, rects]);
  const projectedKeys = useMemo(() => new Set(projected.map(edgeKey)), [projected]);
  // 경로 문자열 캐시: 연결점이 그대로인 간선은 다시 계산하지 않는다(끌기 중에는 움직인 카드에 닿은 간선만 바뀐다).
  // 이번 프레임 간선만 새 맵으로 옮겨 크기가 간선 수를 넘지 않는다.
  const pathCache = useRef<{ style: LinkStyle; map: Map<string, { sig: string; d: string }> }>({ style: linkStyle, map: new Map() });
  const paths = useMemo(() => {
    const previous = pathCache.current.style === linkStyle ? pathCache.current.map : new Map<string, { sig: string; d: string }>();
    const next = new Map<string, { sig: string; d: string }>();
    const draw = PATH_STRATEGIES[linkStyle];
    for (const item of anchors) {
      const sig = anchorSignature(item);
      const hit = previous.get(item.key);
      next.set(item.key, hit && hit.sig === sig ? hit : { sig, d: draw(item) });
    }
    pathCache.current = { style: linkStyle, map: next };
    return next;
  }, [anchors, linkStyle]);
  const doneIds = useMemo(() => new Set(allNotes.filter((note) => note.canvas.done).map((note) => note.id)), [allNotes]);
  const width = 1.5 / Math.max(zoom, .45);
  return <svg className="canvas-links" aria-hidden="true" data-link-style={linkStyle}>
    <defs>
      {/* 꺾쇠 화살촉: 선 굵기 단위라 줌·강조에 따라 같은 비율로 커진다. */}
      <marker id="arrow" viewBox="0 0 10 10" markerWidth="5.4" markerHeight="5.4" refX="2" refY="5" orient="auto" markerUnits="strokeWidth">
        <path d="M2 1.5 L8 5 L2 8.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/>
      </marker>
    </defs>
    {anchors.map((item) => {
      const [fromId, toId] = item.key.split(':');
      const isDirect = !projectedKeys.has(item.key);
      const sample = isDirect ? sampleForLink(item.key) : RESTING;
      const target = isDirect && sample.pulsing && !sample.creating && sample.travel >= 1 ? positions[toId] : undefined;
      return <LinkItem key={item.key} d={paths.get(item.key)?.d ?? ''} isDirect={isDirect} sample={sample} width={width} zoom={zoom}
        highlighted={selected.includes(fromId) || selected.includes(toId)} dimmed={focus ? !(focus.has(fromId) && focus.has(toId)) : false}
        arrive={target} arriveDone={target ? doneIds.has(toId) : false}/>;
    })}
  </svg>;
});

/** 연결점이 같은지 가르는 문자열. 같은 서명이면 같은 경로다. */
const anchorSignature = ({ from, to, bend }: EdgeAnchors) => `${from.side}${from.point.x},${from.point.y}${to.side}${to.point.x},${to.point.y}/${bend}`;

/** 간선 하나. 값만 받아 memo로 바뀐 간선만 다시 그린다. */
const LinkItem = memo(function LinkItem({ d, isDirect, sample, width, zoom, highlighted, dimmed, arrive, arriveDone }: {
  d: string; isDirect: boolean; sample: LinkSample; width: number; zoom: number; highlighted: boolean; dimmed: boolean; arrive?: CanvasPoint; arriveDone: boolean;
}) {
  return <g className={`canvas-link${highlighted ? ' is-highlighted' : ''}${dimmed ? ' is-dimmed' : ''}`}>
    <path d={d} markerEnd="url(#arrow)" pathLength={isDirect ? 1 : undefined} strokeWidth={highlighted ? width * 1.35 : width}
      strokeDasharray={!isDirect ? '5 5' : sample.creating ? '1' : sample.pulsing ? '.12 .88' : undefined}
      strokeDashoffset={sample.creating ? 1 - sample.progress : sample.pulsing ? -sample.travel : undefined}
      opacity={sample.pulsing ? sample.alpha : undefined}/>
    {arrive && <rect x={arrive.x - 4} y={arrive.y - 4} width={CARD_WIDTH + 8} height={CARD_HEIGHT + 8} rx="14" fill="none" className={arriveDone ? 'link-arrive is-done' : 'link-arrive'} strokeWidth={2.5 / Math.max(zoom, .45)} opacity={sample.alpha * .85}/>}
  </g>;
});
