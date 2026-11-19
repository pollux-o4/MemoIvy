import { createContext, type ReactNode, useContext, useMemo } from 'react';
import type { Note } from './engine_contract';
import { createProgress, type ProgressIndex } from './progress';

const ProgressContext = createContext<ProgressIndex | null>(null);

/** 화면마다 메모 목록 한 벌로 진행 집계를 한 번 만들어 카드·드로어·목록이 나눠 쓴다. */
export function ProgressProvider({ notes, children }: { notes: readonly Note[]; children: ReactNode }) {
  const progress = useMemo(() => createProgress(notes), [notes]);
  return <ProgressContext.Provider value={progress}>{children}</ProgressContext.Provider>;
}

export function useProgress(): ProgressIndex {
  const progress = useContext(ProgressContext);
  if (!progress) throw new Error('ProgressProvider 안에서만 진행을 읽을 수 있습니다.');
  return progress;
}
