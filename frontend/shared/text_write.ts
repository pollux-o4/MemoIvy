import type { Snapshot } from './engine_contract';

/** 본문 충돌의 사용자 문구(SSoT). 엔진 원문은 화면에 노출하지 않는다. */
export const TEXT_CONFLICT_MESSAGE = '다른 창에서도 이 메모를 고쳤어요. 어느 쪽 내용을 남길까요?';
const ENGINE_CONFLICT_MESSAGE = 'This note changed in another window.';

export const isConflict = (reason: unknown) => (reason as { code?: string } | null)?.code === 'CONFLICT';
export const userMessage = (message: string) => message === ENGINE_CONFLICT_MESSAGE ? TEXT_CONFLICT_MESSAGE : message;

/**
 * 낙관적 본문 쓰기의 재기준 규칙. 엔진은 메모 단위 revision으로 충돌을 판정해 위치·크기 같은 본문 외 변경도 충돌로 본다.
 * 최신 본문이 편집 출발점(origin)과 같으면 본문 충돌이 아니므로 최신 revision으로 다시 쓸 수 있다. O(메모 수).
 */
export function rebaseRevision(latest: Snapshot | undefined, noteId: string, origin: string): number | undefined {
  const remote = latest?.notes.find((item) => item.id === noteId);
  return latest && remote?.text === origin ? latest.revision : undefined;
}
