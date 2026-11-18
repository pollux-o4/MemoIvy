import { Icon } from '../../../shared/Icon';
import type { Note } from '../../../shared/engine_contract';
import { canvasTitle } from '../model/labels';

export function CanvasNavigator({ path, shortcuts, scopedProject, onNavigate, onTogglePin }: {
  path: Note[];
  shortcuts: Note[];
  scopedProject?: Note;
  onNavigate: (id: string | null) => void;
  onTogglePin: (project: Note) => void;
}) {
  return <nav className="canvas-navigator-row" aria-label="프로젝트 경로">
    <div className="canvas-navigator floating">
      <button className="canvas-home" title="최상위 보드" aria-current={path.length === 0 ? 'page' : undefined} onClick={() => onNavigate(null)}><Icon name="home" size={16}/><span>전체</span></button>{path.map((item, index) => <span key={item.id}><i>›</i><button title={index === path.length - 1 ? '나가기 · Esc 또는 Alt+↑' : canvasTitle(item)} aria-current={index === path.length - 1 ? 'page' : undefined} onClick={() => onNavigate(item.id)}>{canvasTitle(item)}</button></span>)}
      {scopedProject && <button className="canvas-pin" title={scopedProject.canvas.pinned ? '바로가기 고정 해제' : '바로가기에 고정'} aria-label={scopedProject.canvas.pinned ? '바로가기 고정 해제' : '바로가기에 고정'} onClick={() => onTogglePin(scopedProject)}>{scopedProject.canvas.pinned ? '★' : '☆'}</button>}
      {shortcuts.map((project) => <button className="canvas-shortcut" key={project.id} title={`${canvasTitle(project)} · ${project.canvas.pinned ? '고정한 바로가기' : '자주 방문'}`} onClick={() => onNavigate(project.id)}>{project.canvas.pinned ? '★ ' : ''}{canvasTitle(project)}</button>)}
    </div>
  </nav>;
}
