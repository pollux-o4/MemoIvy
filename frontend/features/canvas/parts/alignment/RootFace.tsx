import { Icon } from '../../../../shared/Icon';
import { ALIGN_GROUPS, type AlignGroup } from '../../model/alignment_actions';

/** 첫 면: 선택 수, 묶음 입구, 바로 실행할 일. 묶음 안 동작은 여기서 그리지 않는다. */
export function RootFace({ count, returned, onOpen, onDelete, onGroup, onLift }: {
  count: number; returned: boolean; onOpen: (id: AlignGroup['id']) => void; onDelete: () => void; onGroup: () => void; onLift?: () => void;
}) {
  return <div className="alignment-face" data-dir={returned ? 'out' : undefined}>
    <span className="alignment-count">{count}개 선택</span>
    {ALIGN_GROUPS.map((group) => <button key={group.id} className="alignment-group" data-group={group.id} title={`${group.label} 도구`} onClick={() => onOpen(group.id)}>
      <Icon name={group.icon} size={15}/><span>{group.label}</span><Icon name="chevronRight" size={12}/>
    </button>)}
    <i/>
    <button title="새 프로젝트로 묶기 · Alt+G" aria-label="새 프로젝트로 묶기" onClick={onGroup}><Icon name="project" size={15}/></button>
    {onLift && <button title="프로젝트 밖으로 꺼내기 · Alt+Shift+G" aria-label="프로젝트 밖으로 꺼내기" onClick={onLift}><Icon name="up" size={15}/></button>}
    <button className="danger" title="선택한 메모 삭제 · Delete" aria-label="선택한 메모 삭제" onClick={onDelete}><Icon name="trash" size={15}/></button>
  </div>;
}
