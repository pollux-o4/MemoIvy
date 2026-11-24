import type { PointerEvent } from 'react';
import { Icon } from '../../../../shared/Icon';
import type { PreviewItem } from '../../../../shared/markdown/preview/model';
import { openActionTitle, openActions } from '../../model/open_actions';

/** 들어가기 버튼의 이름·툴팁은 카드 메뉴·인스펙터와 같은 openActions에서 온다. */
const ENTER = openActions(true)[0];

const stop = (event: PointerEvent) => event.stopPropagation();

/** 카드 머리: 프로젝트 표시·제목 표식·제목과 들어가기·완료 버튼. 버튼 누름은 카드 끌기로 번지지 않는다. */
export function CardHeader({ title, marker, project, done, onEnter, onToggleDone }: { title: string; marker: PreviewItem; project: boolean; done: boolean; onEnter: () => void; onToggleDone: () => void }) {
  const doneLabel = done ? '완료 취소' : '완료로 표시';
  return <header>
    {project && <span className="canvas-project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>}
    {marker.kind === 'task' && <span className="canvas-title-marker" aria-hidden="true">{marker.task === 'x' ? '☑' : '☐'}</span>}
    {marker.kind === 'number' && <span className="canvas-title-marker" aria-hidden="true">{marker.mark}</span>}
    {marker.kind === 'bullet' && <span className="canvas-title-marker" aria-hidden="true">•</span>}
    <h3 className={marker.kind === 'task' && marker.task === 'x' ? 'line-done' : ''}>{title}</h3>
    {project && <button className="card-enter" title={openActionTitle(ENTER)} aria-label={`${title} ${ENTER.label}`} onPointerDown={stop} onClick={onEnter}><Icon name="arrow" size={16}/></button>}
    <button className="card-done" title={doneLabel} aria-label={doneLabel} onPointerDown={stop} onClick={(event) => { event.stopPropagation(); onToggleDone(); }}><Icon name="check" size={14}/></button>
  </header>;
}
