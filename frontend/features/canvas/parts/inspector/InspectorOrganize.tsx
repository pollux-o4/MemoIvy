import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { CanvasProjectMenu } from '../CanvasProjectMenu';

/** 4층 · 정리: 다른 프로젝트로 옮기기·밖으로 꺼내기. 할 수 있는 게 없으면 통째로 숨긴다. */
export function InspectorOrganize({ projects, onLift, onMoveInto }: { projects: Note[]; onLift?: () => void; onMoveInto: (projectId: string) => void }) {
  if (!projects.length && !onLift) return null;
  return <section className="inspector-group" aria-label="정리">
    <header><b>정리</b></header>
    <CanvasProjectMenu projects={projects} onMove={onMoveInto}/>
    {onLift && <button className="inspector-action" title="한 단계 위 보드로 옮기기 · Alt+Shift+G" onClick={onLift}><Icon name="up" size={15}/><span>밖으로 꺼내기</span></button>}
  </section>;
}
