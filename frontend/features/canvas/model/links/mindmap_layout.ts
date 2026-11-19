import { CARD_HEIGHT, CARD_WIDTH } from '../align';
import type { CanvasPoint } from './types';

export interface MindmapOptions {
  columnGap?: number;
  rowGap?: number;
}

interface LayoutFrame {
  id: string;
  depth: number;
  expanded: boolean;
  children: string[];
}

const DEFAULT_COLUMN_GAP = 80;
const DEFAULT_ROW_GAP = 24;

/** 루트에서 오른쪽으로 뻗는 마인드맵 트리 좌표를 계산한다. */
export function mindmapLayout(
  rootId: string,
  children: (id: string) => readonly string[],
  sizes: Record<string, { w: number; h: number }>,
  origin: CanvasPoint,
  options?: MindmapOptions,
): Record<string, CanvasPoint> {
  const columnGap = options?.columnGap ?? DEFAULT_COLUMN_GAP;
  const rowGap = options?.rowGap ?? DEFAULT_ROW_GAP;
  const sizeOf = (id: string) => sizes[id] ?? { w: CARD_WIDTH, h: CARD_HEIGHT };

  const visited = new Set<string>([rootId]);
  const nodes: Array<{ id: string; depth: number }> = [];
  const maxWidth: number[] = [];
  const centerY = new Map<string, number>();
  const stack: LayoutFrame[] = [{ id: rootId, depth: 0, expanded: false, children: [] }];
  let cursorY = 0;

  while (stack.length > 0) {
    const frame = stack[stack.length - 1];
    if (!frame.expanded) {
      frame.expanded = true;
      nodes.push({ id: frame.id, depth: frame.depth });
      const { w, h } = sizeOf(frame.id);
      maxWidth[frame.depth] = Math.max(maxWidth[frame.depth] ?? 0, w);

      const nextChildren: string[] = [];
      for (const childId of children(frame.id)) {
        if (visited.has(childId)) continue;
        visited.add(childId);
        nextChildren.push(childId);
      }
      frame.children = nextChildren;

      if (nextChildren.length === 0) {
        stack.pop();
        centerY.set(frame.id, cursorY + h / 2);
        cursorY += h + rowGap;
      } else {
        for (let i = nextChildren.length - 1; i >= 0; i--) {
          stack.push({ id: nextChildren[i], depth: frame.depth + 1, expanded: false, children: [] });
        }
      }
      continue;
    }

    stack.pop();
    const firstCenter = centerY.get(frame.children[0]) ?? 0;
    const lastCenter = centerY.get(frame.children[frame.children.length - 1]) ?? 0;
    centerY.set(frame.id, (firstCenter + lastCenter) / 2);
  }

  // 깊이별 열 시작 x 계산
  const columnX: number[] = [origin.x];
  for (let d = 1; d < maxWidth.length; d++) {
    columnX[d] = columnX[d - 1] + maxWidth[d - 1] + columnGap;
  }

  // 루트 y가 origin.y가 되도록 전체 세로 이동
  const rootRawY = (centerY.get(rootId) ?? 0) - sizeOf(rootId).h / 2;
  const result: Record<string, CanvasPoint> = {};
  for (const { id, depth } of nodes) {
    const rawY = (centerY.get(id) ?? 0) - sizeOf(id).h / 2;
    result[id] = { x: columnX[depth], y: origin.y + (rawY - rootRawY) };
  }
  return result;
}
