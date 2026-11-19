import type { SnapGuide } from '../model/snap';

/** 맞닿은 카드 끝을 넘어 화면 기준 이만큼(px) 더 그어 붙은 위치를 드러낸다. */
const OVERHANG_PX = 12;

/** 카드 끌기 중 붙은 선. 월드 좌표에 그리고, 확대율과 무관하게 화면에서 1px 굵기를 유지한다. */
export function SnapGuides({ guides, zoom }: { guides: SnapGuide[]; zoom: number }) {
  if (!guides.length) return null;
  const extra = OVERHANG_PX / zoom;
  return <svg className="canvas-guides" aria-hidden="true" style={{ strokeWidth: 1 / zoom }}>
    {guides.map((guide) => guide.axis === 'x'
      ? <line key="x" x1={guide.at} x2={guide.at} y1={guide.from - extra} y2={guide.to + extra}/>
      : <line key="y" y1={guide.at} y2={guide.at} x1={guide.from - extra} x2={guide.to + extra}/>)}
  </svg>;
}
