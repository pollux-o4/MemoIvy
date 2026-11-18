import { useEffect, useMemo, useRef, useState } from 'react';
import { createAutosave, LIVE_SAVE, type SavePolicy } from './autosave';

/** 저장 함수. 기준 버전을 받아 저장 뒤 새 버전을 돌려주고, 실패(충돌 등)는 reject한다. */
export type DraftCommit = (value: string, baseVersion?: number) => Promise<number | undefined> | undefined;

export interface DraftOptions {
  /** 저장값의 버전. 편집 시작 때 값을 기준 버전으로 넘겨 다른 곳의 수정과 충돌을 감지한다. */
  version?: number;
  /** 주면 입력 중에도 이 정책대로 저장한다. 없으면 마칠 때만 저장한다. */
  live?: SavePolicy | false;
}

/** 입력 장치와 무관한 초안 조작. 입력 칸·편집기 바인딩이 이 위에 얹힌다. */
export interface Draft {
  value: string;
  change: (value: string) => void;
  /** 한글 등 조합 시작·끝. 조합 중에는 저장하지 않는다. */
  compose: (active: boolean, value?: string) => void;
  /** 편집을 마치고 값을 바로 저장한다. */
  finish: (value: string) => void;
  /** 편집 시작 때 값으로 되돌리고 그 값을 돌려준다. */
  revert: () => string;
}

/**
 * 저장값과 분리된 초안. 편집 중에는 밖에서 온 값으로 초안을 덮지 않아 깜박이지 않는다.
 * 충돌·실패면 저장된 값을 따르고 편집을 끝내 보이는 초안이 저장되지 않은 채 남지 않게 한다.
 */
export function useDraft(saved: string, onCommit: DraftCommit, { live = LIVE_SAVE, version }: DraftOptions = {}): Draft {
  const policy = live || undefined;
  const [draft, setDraft] = useState(saved);
  const editing = useRef(false);
  const composing = useRef(false);
  const origin = useRef(saved);
  const lastSent = useRef(saved);
  const commitRef = useRef(onCommit);
  commitRef.current = onCommit;
  const savedRef = useRef(saved);
  savedRef.current = saved;
  // 기준 버전: 편집하지 않을 때는 최신, 편집 중에는 시작 때 값이며 내 저장이 성공해야만 올린다.
  const base = useRef(version);
  useEffect(() => { if (!editing.current) base.current = version; }, [version]);
  useEffect(() => { lastSent.current = saved; if (!editing.current) setDraft(saved); }, [saved]);

  const reject = () => { autosave?.cancel(); editing.current = false; lastSent.current = savedRef.current; setDraft(savedRef.current); };
  const send = (value: string) => {
    if (value === lastSent.current) return;
    lastSent.current = value;
    const result = commitRef.current(value, base.current);
    if (result) result.then((next) => { if (next !== undefined) base.current = next; }, reject);
  };
  // biome-ignore lint/correctness/useExhaustiveDependencies: 정책이 바뀔 때만 스케줄러를 새로 만든다
  const autosave = useMemo(() => policy ? createAutosave(policy, send) : undefined, [policy?.idle, policy?.maxWait]);
  useEffect(() => () => autosave?.flush(), [autosave]);

  const begin = () => { if (!editing.current) { editing.current = true; origin.current = lastSent.current; } };
  const finish = (value: string) => { autosave?.cancel(); editing.current = false; setDraft(value); send(value); };
  return {
    value: draft,
    change: (value) => { begin(); setDraft(value); if (!composing.current) autosave?.schedule(value); },
    compose: (active, value) => { composing.current = active; if (!active && value !== undefined) autosave?.schedule(value); },
    finish,
    revert: () => { finish(origin.current); return origin.current; },
  };
}
