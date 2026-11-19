import type { Note } from '../../../../shared/engine_contract';
import type { Edge, OutlineRow } from './types';

export interface LinkGraph {
  out: ReadonlyMap<string, readonly string[]>;
  in: ReadonlyMap<string, readonly string[]>;
}

function appendAdj(map: Map<string, string[]>, key: string, value: string): void {
  const list = map.get(key);
  if (list) {
    list.push(value);
  } else {
    map.set(key, [value]);
  }
}

function walkDirection(
  adj: ReadonlyMap<string, readonly string[]>,
  start: string,
  result: Set<string>,
): void {
  const seen = new Set<string>([start]);
  const stack = [start];
  while (stack.length > 0) {
    const cur = stack.pop();
    if (cur === undefined) continue;
    result.add(cur);
    for (const next of adj.get(cur) ?? []) {
      if (seen.has(next)) continue;
      seen.add(next);
      stack.push(next);
    }
  }
}

/** 간선 목록으로 out/in 인접 목록 그래프를 O(E)로 만든다. */
export function buildGraph(edges: Edge[]): LinkGraph {
  const outMap = new Map<string, string[]>();
  const inMap = new Map<string, string[]>();
  for (const edge of edges) {
    appendAdj(outMap, edge.from, edge.to);
    appendAdj(inMap, edge.to, edge.from);
  }
  return { out: outMap, in: inMap };
}

/** 같은 화면 단계(parent === scope)에 속한 메모들 사이의 next 간선만 뽑는다. */
export function scopeEdges(notes: Note[], scope: string | null): Edge[] {
  const scopedNotes: Note[] = [];
  const scopedIds = new Set<string>();
  for (const note of notes) {
    if ((note.canvas.parent ?? null) !== scope) continue;
    scopedNotes.push(note);
    scopedIds.add(note.id);
  }

  const edges: Edge[] = [];
  for (const note of scopedNotes) {
    for (const to of note.canvas.next) {
      if (scopedIds.has(to)) {
        edges.push({ from: note.id, to });
      }
    }
  }
  return edges;
}

/** id 자신과 모든 조상(in), 모든 자손(out) 집합을 구한다. */
export function branchSet(graph: LinkGraph, id: string): Set<string> {
  const result = new Set<string>();
  walkDirection(graph.in, id, result);
  walkDirection(graph.out, id, result);
  return result;
}

/** 여러 부모가 있으면 in 목록의 첫 번째 부모 id를 돌려준다. */
export function treeParent(graph: LinkGraph, id: string): string | undefined {
  return graph.in.get(id)?.[0];
}

/** treeParent를 따라 올라간 꼭대기 id를 구하며 순환이면 재방문 직전에 멈춘다. */
export function branchRoot(graph: LinkGraph, id: string): string {
  const seen = new Set<string>([id]);
  let cur = id;
  while (true) {
    const parent = treeParent(graph, cur);
    if (parent === undefined || seen.has(parent)) {
      return cur;
    }
    seen.add(parent);
    cur = parent;
  }
}

/** rootId부터 out 방향 깊이 우선으로 중복 없는 가지 개요를 만든다. */
export function outline(
  graph: LinkGraph,
  rootId: string,
  isProject: (id: string) => boolean,
): OutlineRow[] {
  const rows: OutlineRow[] = [];
  // 개요 안의 부모는 "이 가지 안에 있는 첫 부모". 가지 밖 부모가 먼저 있어도 노드가 빠지지 않게 한다.
  const reachable = new Set<string>();
  walkDirection(graph.out, rootId, reachable);
  const ownerOf = (node: string) => graph.in.get(node)?.find((parent) => reachable.has(parent));
  const visited = new Set<string>([rootId]);
  const stack: Array<{ id: string; depth: number }> = [{ id: rootId, depth: 0 }];

  while (stack.length > 0) {
    const item = stack.pop();
    if (!item) continue;
    const { id, depth } = item;
    const children: string[] = [];
    for (const next of graph.out.get(id) ?? []) {
      if (visited.has(next)) continue;
      if (ownerOf(next) !== id) continue;
      visited.add(next);
      children.push(next);
    }

    rows.push({
      id,
      depth,
      hasChildren: children.length > 0,
      project: isProject(id),
    });

    for (let i = children.length - 1; i >= 0; i--) {
      stack.push({ id: children[i], depth: depth + 1 });
    }
  }

  return rows;
}
