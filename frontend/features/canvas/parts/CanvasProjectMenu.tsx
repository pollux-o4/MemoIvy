import { useState } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { useMenu } from '../../../shared/menu/useMenu';
import { canvasTitle } from '../model/labels';
import { canvasPreview } from '../model/preview';

export function CanvasProjectMenu({ projects, onMove }: { projects: Note[]; onMove: (projectId: string) => void }) {
  const [open, setOpen] = useState(false);
  const { root, trigger } = useMenu(open, () => setOpen(false));
  if (!projects.length) return null;
  return <div className="canvas-project-menu menu-anchor" ref={root}>
    <button ref={trigger} className="inspector-action" aria-haspopup="menu" aria-expanded={open} title="카드를 프로젝트 카드 위에 끌어 놓아도 옮겨져요" onClick={() => setOpen(!open)}>프로젝트로 옮기기</button>
    {open && <div className="menu-surface menu-popover" role="menu" aria-label="옮길 프로젝트 선택">
      {projects.map((project) => {
        const hint = canvasPreview(project.text, 1).rows[0]?.text;
        return <button className="menu-row two-line" role="menuitem" key={project.id} onClick={() => { setOpen(false); onMove(project.id); }}>
          <span className={`inspector-dot ${project.color}`} aria-hidden="true"/>
          <span className="project-menu-copy"><span>{canvasTitle(project)}</span>{hint && <small className="menu-hint">{hint}</small>}</span>
        </button>;
      })}
    </div>}
  </div>;
}
