import { Icon } from '../../../shared/Icon';
import type { Alignment } from '../model/align';

export function AlignmentBar({ count, onAlign, onDelete, onGroup, onLift }: { count: number; onAlign: (kind: Alignment) => void; onDelete: () => void; onGroup: () => void; onLift?: () => void }) {
  return <div className="alignment-bar" role="toolbar" aria-label="선택 항목 정렬">
    <span>{count}개 선택</span>
    <button title="왼쪽 맞춤 · Alt+A" onClick={() => onAlign('left')}>⇤</button>
    <button title="가로 가운데 맞춤 · Alt+H" onClick={() => onAlign('center')}>↔</button>
    <button title="오른쪽 맞춤 · Alt+D" onClick={() => onAlign('right')}>⇥</button>
    <i/>
    <button title="위쪽 맞춤 · Alt+W" onClick={() => onAlign('top')}>↑</button>
    <button title="세로 가운데 맞춤 · Alt+V" onClick={() => onAlign('middle')}>↕</button>
    <button title="아래쪽 맞춤 · Alt+S" onClick={() => onAlign('bottom')}>↓</button>
    <i/>
    <button title="가로 간격 같게 · Alt+Shift+H" onClick={() => onAlign('distribute-x')} disabled={count < 3}>↔</button>
    <button title="세로 간격 같게 · Alt+Shift+V" onClick={() => onAlign('distribute-y')} disabled={count < 3}>↕</button>
    <button title="격자로 정리 · Alt+T" onClick={() => onAlign('grid')}><Icon name="align" size={15}/></button>
    <button title="연결 순서로 배치 · Alt+F" onClick={() => onAlign('flow')}><Icon name="flow" size={15}/></button>
    <i/>
    <button title="새 프로젝트로 묶기 · Alt+G" onClick={onGroup}><Icon name="project" size={15}/></button>
    {onLift && <button title="프로젝트 밖으로 꺼내기 · Alt+Shift+G" onClick={onLift}><Icon name="up" size={15}/></button>}
    <i/>
    <button className="danger" title="선택한 메모 삭제 · Delete" onClick={onDelete}><Icon name="trash" size={15}/></button>
  </div>;
}
