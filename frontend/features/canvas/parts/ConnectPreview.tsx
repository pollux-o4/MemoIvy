import { cardRect } from '../model/align';
import { PATH_STRATEGIES } from '../model/links/paths';
import { previewAnchors } from '../model/links/preview';
import type { CanvasPoint, LinkStyle } from '../model/links/types';

/** 끌어서 잇는 중의 미리보기 선. 실제 선과 같은 모양 Strategy로 그리고, 놓을 카드가 없으면 끝에 새 가지 자리를 점선 상자로 보인다. */
export function ConnectPreview({ from, to, target, linkStyle, zoom }: {
  from: CanvasPoint; to: CanvasPoint; target?: CanvasPoint; linkStyle: LinkStyle; zoom: number;
}) {
  const d = PATH_STRATEGIES[linkStyle](previewAnchors(cardRect(from), target ? cardRect(target) : to));
  const ghost = cardRect({ x: to.x, y: to.y - cardRect(to).h / 2 });
  return <svg className="canvas-connect-preview" aria-hidden="true">
    <path d={d} strokeWidth={2 / zoom}/>
    {!target && <rect className="connect-ghost" x={ghost.x} y={ghost.y} width={ghost.w} height={ghost.h} rx={12} strokeWidth={1.5 / zoom}/>}
  </svg>;
}
