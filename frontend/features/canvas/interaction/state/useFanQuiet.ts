import { useCallback, useState } from 'react';

/**
 * 프로젝트 부채를 잠시 재우는 카드 목록. 클릭(누름)으로는 부채를 건드리지 않고,
 * 카드를 실제로 끌었을 때만 재운 뒤 포인터가 그 카드를 떠날 때 깨운다(같은 커서 아래에서 모션을 다시 보이지 않는다).
 */
export function useFanQuiet() {
  const [quiet, setQuiet] = useState<readonly string[]>([]);
  const hush = useCallback((ids: readonly string[]) => setQuiet((current) => ids.every((id) => current.includes(id)) ? current : [...new Set([...current, ...ids])]), []);
  const wake = useCallback((id: string) => setQuiet((current) => current.includes(id) ? current.filter((item) => item !== id) : current), []);
  return { quiet, hush, wake };
}
