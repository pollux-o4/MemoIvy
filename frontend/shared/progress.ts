import type { Note } from './engine_contract';
import { type ChildIndex, createChildIndex } from './hierarchy';
import { createListItemReader } from './markdown/list_item';

/** 완료 수 / 전체 수. 할 일·하위 항목 모두 같은 모양이다. */
export type Progress = { done: number; total: number };

const NONE: Progress = { done: 0, total: 0 };

/** 본문의 체크박스 할 일. 줄 문법은 markdown/list_item이 단일 출처다. */
function checklist(markdown: string): Progress {
  let done = 0, total = 0;
  const readItem = createListItemReader();
  for (const line of markdown.split('\n')) {
    const task = readItem(line)?.task;
    if (!task) continue;
    total++; if (task === 'x') done++;
  }
  return { done, total };
}

export interface ProgressIndex {
  /** 그 단계에서 보이는 진행. 메모는 자기 할 일, 프로젝트는 자기 할 일 + 바로 아래 항목을 한 단위씩. */
  of: (note: Note) => Progress;
  /** 하위 전체 체크박스 합계. 단계 진행의 보조 설명용. */
  detail: (note: Note) => Progress;
  /** 집계가 기준으로 삼는 바로 아래 메모. 개수·부채 표시도 같은 단계를 쓴다. */
  children: ChildIndex;
}

/**
 * 단계별 진행 집계(Composite). 프로젝트는 바로 아래 메모·프로젝트를 한 단위로 세어, 할 일이 수백 개여도
 * 상위에서는 "하위 항목 몇 개가 끝났나"만 보인다. 단위는 완료 표시가 있거나 자기 진행이 모두 끝나면 완료이고,
 * 할 일도 완료 표시도 없는 참고용 메모는 세지 않는다. 메모 목록 한 벌로 만들어 결과를 기억한다.
 */
export function createProgress(notes: readonly Note[]): ProgressIndex {
  const children = createChildIndex(notes);
  const levels = new Map<string, Progress>();
  const details = new Map<string, Progress>();
  const visiting = new Set<string>(); // 부모 순환을 끊는다.

  const add = (a: Progress, b: Progress): Progress => ({ done: a.done + b.done, total: a.total + b.total });
  const unit = (child: Note): Progress => {
    if (child.canvas.done) return { done: 1, total: 1 };
    const own = of(child);
    return own.total === 0 ? NONE : { done: own.done === own.total ? 1 : 0, total: 1 };
  };
  const of = (note: Note): Progress => {
    const cached = levels.get(note.id);
    if (cached) return cached;
    if (!note.canvas.project) return checklist(note.text);
    if (visiting.has(note.id)) return NONE;
    visiting.add(note.id);
    const result = children(note.id).filter((child) => !visiting.has(child.id)).map(unit).reduce(add, checklist(note.text));
    visiting.delete(note.id);
    levels.set(note.id, result);
    return result;
  };
  const detail = (note: Note): Progress => {
    const cached = details.get(note.id);
    if (cached) return cached;
    if (visiting.has(note.id)) return NONE;
    visiting.add(note.id);
    const result = children(note.id).map(detail).reduce(add, checklist(note.text));
    visiting.delete(note.id);
    details.set(note.id, result);
    return result;
  };
  return { of, detail, children };
}

/** 진행 표시 문구의 단일 출처. 프로젝트는 하위 항목 단위, 메모는 할 일 단위로 말한다. */
export function progressLabel(note: Note, progress: ProgressIndex): { short: string; title: string } {
  const level = progress.of(note);
  const short = `${level.done}/${level.total}`;
  if (!note.canvas.project) return { short, title: `할 일 ${level.total}개 중 ${level.done}개 완료` };
  const all = progress.detail(note);
  return { short, title: `하위 항목 ${level.total}개 중 ${level.done}개 완료 · 할 일 ${all.total}개 중 ${all.done}개` };
}
