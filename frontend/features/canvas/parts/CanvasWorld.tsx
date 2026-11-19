import { type CSSProperties, memo, useRef } from 'react';
import type { LinkStyle } from '../../../shared/link_style';
import type { Note } from '../../../shared/engine_contract';
import type { CanvasPoint } from '../model/align';
import { gridStyle } from '../model/grid';
import type { Camera } from '../model/motion';
import type { SnapGuide } from '../model/snap';
import { type CardActions, CanvasCard } from './CanvasCard';
import { CanvasLinks } from './CanvasLinks';
import { ConnectPreview } from './ConnectPreview';
import { SnapGuides } from './SnapGuides';
import { useCardHeights } from './useCardHeights';

type LinkSampler = Parameters<typeof CanvasLinks>[0]['sampleForLink'];
export type WorldConnect = { from: CanvasPoint; to: CanvasPoint; target?: CanvasPoint; targetId?: string };

/**
 * 카메라가 비추는 캔버스 세계: 격자·연결선·카드·잇기 미리보기·정렬 안내선. 카드의 실제 높이는 연결선에 전달한다.
 * 가지 초점 흐림은 world의 has-focus가 정하고 카드·선은 가지 안인지만 받는다.
 */
export const CanvasWorld = memo(function CanvasWorld(o: {
  camera: Camera; notes: Note[]; allNotes: Note[]; scope: string | null; positions: Record<string, CanvasPoint>;
  selected: string[]; focus?: ReadonlySet<string>; quiet: readonly string[]; dropTarget: string | null;
  connect: WorldConnect | null; guides: SnapGuide[]; linkStyle: LinkStyle; sampleForLink: LinkSampler; actions: CardActions;
}) {
  const { camera } = o;
  const world = useRef<HTMLDivElement>(null);
  const heights = useCardHeights(world);
  return <>
    <div className="canvas-grid" aria-hidden="true" style={gridStyle(camera) as CSSProperties}/>
    <div ref={world} className={`canvas-world${o.focus ? ' has-focus' : ''}`} style={{ transform: `translate(${-camera.x * camera.zoom}px, ${-camera.y * camera.zoom}px) scale(${camera.zoom})` }}>
      <CanvasLinks notes={o.notes} allNotes={o.allNotes} scope={o.scope} positions={o.positions} heights={heights} zoom={camera.zoom} selected={o.selected} sampleForLink={o.sampleForLink} linkStyle={o.linkStyle} focus={o.focus}/>
      {o.notes.map((note) => <CanvasCard key={note.id} note={note} position={o.positions[note.id]} selected={o.selected.includes(note.id)} dropTarget={o.dropTarget === note.id}
        fanQuiet={o.quiet.includes(note.id)} linkTarget={o.connect?.targetId === note.id} focused={Boolean(o.focus?.has(note.id))} actions={o.actions}/>)}
      {o.connect && <ConnectPreview from={o.connect.from} to={o.connect.to} target={o.connect.target} linkStyle={o.linkStyle} zoom={camera.zoom}/>}
      <SnapGuides guides={o.guides} zoom={camera.zoom}/>
    </div>
  </>;
});
