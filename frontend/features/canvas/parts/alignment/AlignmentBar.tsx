import { useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { Alignment } from '../../model/align';
import { ALIGN_GROUPS, type AlignGroup } from '../../model/alignment_actions';
import { GroupFace } from './GroupFace';
import { RootFace } from './RootFace';
import { useFaceFocus } from './useFaceFocus';

type Face = 'root' | AlignGroup['id'];

/**
 * 다중 선택 막대(CANVAS-003). 면 상태만 가지며 그리기는 각 면이 맡는다(State).
 * 위에서 아래로: 첫 면 → 묶음 면. 같은 표면이 폭을 바꾸며 모양을 잇고(canvas.css), Esc·뒤로는 한 단계만 올라간다.
 */
export function AlignmentBar({ count, onAlign, onDelete, onGroup, onLift }: { count: number; onAlign: (kind: Alignment) => void; onDelete: () => void; onGroup: () => void; onLift?: () => void }) {
  const [face, setFace] = useState<Face>('root');
  const { bar, leaving, returned } = useFaceFocus(face, 'root');
  const go = (next: Face) => { leaving(face); setFace(next); };
  const group = ALIGN_GROUPS.find((item) => item.id === face);
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape' || !group) return;
    event.stopPropagation(); // 캔버스의 Esc(선택 해제)보다 먼저 한 단계만 올라간다.
    go('root');
  };
  return <div ref={bar} className="alignment-bar" role="toolbar" aria-label={group ? `선택 항목 ${group.label}` : '선택 항목 정렬'} onKeyDown={onKeyDown}>
    {group
      ? <GroupFace key={group.id} group={group} count={count} onBack={() => go('root')} onAlign={onAlign}/>
      : <RootFace key="root" count={count} returned={returned} onOpen={go} onDelete={onDelete} onGroup={onGroup} onLift={onLift}/>}
  </div>;
}
