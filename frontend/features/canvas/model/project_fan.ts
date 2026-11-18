import type { Note } from '../../../shared/engine_contract';

/** 부채에 펼칠 최대 장수. 넘치는 수는 "외 N개"로만 알린다(장수가 늘면 읽히지 않는다). */
export const FAN_LIMIT = 3;

export type ProjectFan = { recent: Note[]; more: number };

/** 하위 메모 중 최근 수정 순 상위 limit개와 나머지 수. 입력 배열은 바꾸지 않는다. */
export function projectFan(descendants: readonly Note[], limit = FAN_LIMIT): ProjectFan {
  const recent = [...descendants].sort((a, b) => b.modified - a.modified).slice(0, limit);
  return { recent, more: Math.max(0, descendants.length - recent.length) };
}
