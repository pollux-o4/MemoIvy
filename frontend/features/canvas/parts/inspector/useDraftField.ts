import { type KeyboardEvent, useEffect, useState } from 'react';

/**
 * 저장된 값과 따로 노는 입력 초안. 포커스를 잃거나 Enter(여러 줄이면 Ctrl+Enter)에 저장, Esc에 되돌린다.
 * 다른 메모로 바뀌거나 밖에서 값이 바뀌면 편집 중이 아닐 때만 초안을 맞춘다.
 */
export function useDraftField(saved: string, onCommit: (value: string) => void, multiline = false) {
  const [draft, setDraft] = useState(saved);
  const [editing, setEditing] = useState(false);
  useEffect(() => { if (!editing) setDraft(saved); }, [saved, editing]);
  // 저장은 입력 칸의 현재 값을 읽는다. 입력 직후 Enter·포커스 이동이 렌더보다 빨라도 마지막 글자를 잃지 않는다.
  const commit = (value: string) => { setEditing(false); setDraft(value); if (value !== saved) onCommit(value); };
  return {
    value: draft,
    onChange: (event: { target: { value: string } }) => { setEditing(true); setDraft(event.target.value); },
    onBlur: (event: { currentTarget: { value: string } }) => commit(event.currentTarget.value),
    onKeyDown: (event: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      event.stopPropagation(); // 입력 중 캔버스 단축키(삭제·방향키 등)가 동작하지 않게 한다.
      if (event.key === 'Escape') { event.currentTarget.value = saved; setDraft(saved); setEditing(false); event.currentTarget.blur(); }
      else if (event.key === 'Enter' && (!multiline || event.ctrlKey || event.metaKey)) { event.preventDefault(); event.currentTarget.blur(); }
    },
  };
}
