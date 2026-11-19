/**
 * 캔버스 간선·마인드맵 계산의 공용 계약. 화면(React)을 모르는 순수 데이터만 둔다.
 * 각 계산 모듈(anchors · paths · graph · mindmap_layout)은 이 타입만 주고받는다.
 */
import type { CanvasPoint } from '../align';

export type { CanvasPoint };

/** 카드가 차지하는 사각형(왼쪽 위 기준, 캔버스 좌표). 미리보기를 펼친 카드는 h가 더 크다. */
export interface Rect { x: number; y: number; w: number; h: number }

export type Side = 'left' | 'right' | 'top' | 'bottom';

/** 방향 있는 간선 하나. from → to. */
export interface Edge { from: string; to: string }

/** 간선 끝 한쪽의 연결점. normal은 카드 밖을 향하는 단위 벡터. */
export interface Anchor { side: Side; point: CanvasPoint; normal: CanvasPoint }

/** 간선 하나의 양 끝 연결점. bend는 양방향 간선을 벌릴 때 진행 방향 왼쪽으로 미는 거리(px). 반대 방향 두 간선이 같은 값이면 서로 반대편으로 벌어진다. */
export interface EdgeAnchors { key: string; from: Anchor; to: Anchor; bend: number }

/** 직전 프레임에 고른 변. 키는 edgeKey(edge). 경계 근처에서 변이 깜빡이지 않게 넘긴다. */
export type SideMemory = ReadonlyMap<string, { from: Side; to: Side }>;

export type { LinkStyle } from '../../../../shared/link_style';

/** 선 모양 Strategy: 연결점 두 개로 SVG path d 문자열을 만든다. */
export type PathStrategy = (anchors: EdgeAnchors) => string;

export const edgeKey = (edge: Edge) => `${edge.from}:${edge.to}`;

/** 카드 모서리 반경(canvas.css .canvas-note border-radius)과 여유. 연결점은 이 안쪽으로 들어가지 않는다. */
const CARD_RADIUS = 10;
export const ANCHOR_INSET = CARD_RADIUS + 8;
/** 화살촉 길이(px, 선 굵기 1.5 기준). 경로 끝을 이만큼 줄여 화살촉이 테두리에 닿게 한다. */
export const ARROW_LENGTH = 8;

/** 가지 개요 한 줄. 프로젝트(서브그래프)는 project=true이고 안의 메모는 개요에 펼치지 않는다. */
export interface OutlineRow { id: string; depth: number; hasChildren: boolean; project: boolean }
