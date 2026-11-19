import { Icon } from '../../../../shared/Icon';
import type { Alignment } from '../../model/align';
import { type AlignGroup, chordLabel } from '../../model/alignment_actions';

/** 묶음 면: 뒤로 + 같은 축끼리 모은 동작 행. 조건이 안 되는 동작은 이유를 툴팁에 적고 끈다. */
export function GroupFace({ group, count, onBack, onAlign }: { group: AlignGroup; count: number; onBack: () => void; onAlign: (kind: Alignment) => void }) {
  return <div className="alignment-face" data-dir="in">
    <button className="alignment-back" title="뒤로 · Esc" aria-label="뒤로" onClick={onBack}><Icon name="chevronRight" size={15}/></button>
    <span className="alignment-title">{group.label}</span>
    {group.rows.map((row) => <span key={row[0].kind} className="alignment-row">
      {row.map((action) => {
        const short = count < action.minCount;
        return <button key={action.kind} data-action={action.kind} disabled={short} aria-label={action.label}
          title={`${action.label} · ${chordLabel(action.chord)}${short ? ` · ${action.minCount}개 이상` : ''}`}
          onClick={() => onAlign(action.kind)}><Icon name={action.icon} size={16}/></button>;
      })}
    </span>)}
  </div>;
}
