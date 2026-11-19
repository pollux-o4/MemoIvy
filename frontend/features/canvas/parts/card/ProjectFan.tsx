import type { CSSProperties } from 'react';
import './project_fan.css';
import type { Note } from '../../../../shared/engine_contract';
import { noteColor, projectSheets } from '../../../../shared/model';
import type { CanvasPoint } from '../../model/align';
import { canvasTitle } from '../../model/labels';
import type { ProjectFan as Fan } from '../../model/project_fan';

// 프로젝트 카드 바로 뒤 형제로 그린다(카드는 overflow·stacking context를 가져 안쪽에 둘 수 없다).
// 쉴 때는 오른쪽 아래로 비낀 1~2장, 카드에 올리거나 포커스하면 최근 하위 메모가 위로 부채처럼 떠오른다(비면 빈 종이 1장에 안내). 모션은 CSS만 쓴다.
const SLOTS = ['near', 'mid', 'far'] as const;

export function ProjectFan({ id, fan, position, tone }: { id?: string; fan: Fan; position: CanvasPoint; tone: string }) {
  const resting = projectSheets(fan.recent.length + fan.more);
  const sheets = SLOTS.map((slot, index) => ({ slot, note: fan.recent[index] as Note | undefined, resting: index < resting }))
    .filter((sheet) => sheet.note || sheet.resting);
  return <div id={id} className="canvas-fan" aria-hidden="true" style={{ left: position.x, top: position.y, '--card-tone': tone } as CSSProperties}>
    {sheets.reverse().map(({ slot, note, resting: rest }, index) => <div key={slot} className={`canvas-fan-sheet ${slot}${rest ? ' resting' : ''}`}>
      {note ? <span className="canvas-fan-title" style={{ '--sheet-accent': noteColor(note.color).accent } as CSSProperties}><i/>{canvasTitle(note)}</span>
        : slot === 'near' && fan.recent.length === 0 && <span className="canvas-fan-title empty">아직 메모가 없어요</span>}
      {index === 0 && fan.more > 0 && <small className="canvas-fan-more">외 {fan.more}개</small>}
    </div>)}
  </div>;
}
