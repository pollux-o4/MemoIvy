import { type CSSProperties, memo, type MouseEvent, type PointerEvent, useMemo, useState } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { noteColor } from '../../../shared/model';
import { progressLabel } from '../../../shared/progress';
import { useProgress } from '../../../shared/ProgressProvider';
import type { CanvasPoint } from '../model/align';
import { canvasTitle } from '../model/labels';
import { canvasPreview } from '../model/preview';
import { projectFan } from '../model/project_fan';
import { CardHeader } from './card/CardHeader';
import { CardPreview } from './card/CardPreview';
import { LinkHandle } from './card/LinkHandle';
import { ProjectFan } from './card/ProjectFan';
import { ProjectSummary } from './card/ProjectSummary';

/** 카드가 부모에게 알리는 동작. Canvas가 한 번 만든 고정 객체라 카드 memo를 깨지 않는다. */
export interface CardActions {
  pointerDown: (event: PointerEvent, id: string) => void; pointerLeave: (id: string) => void;
  open: (note: Note) => void; connectStart: (id: string) => void; toggleDone: (id: string) => void;
  contextMenu: (event: MouseEvent, id: string) => void; togglePeek: (id: string) => void;
}

/**
 * 캔버스 카드 조립: 상태 클래스·포인터 연결만 맡고 내용은 card/ 부품이 그린다. 프로젝트면 뒤에 부채를 함께 둔다.
 * memo: 끌기 중에는 위치가 바뀐 카드만 다시 그린다(props는 값·고정 참조만 받는다).
 */
export const CanvasCard = memo(function CanvasCard({ note, position, selected, dropTarget, fanQuiet = false, dimmed = false, peeking, actions }: {
  note: Note; position: CanvasPoint; selected: boolean; dropTarget?: boolean; fanQuiet?: boolean; dimmed?: boolean;
  /** 프로젝트 제자리 펼치기 상태. 프로젝트 카드에만 넘긴다(있으면 하위 메모 수 옆에 펼침 단추). */
  peeking?: boolean; actions: CardActions;
}) {
  const project = Boolean(note.canvas.project);
  const preview = useMemo(() => canvasPreview(note.text), [note.text]);
  const title = preview.title.text || canvasTitle(note);
  const tone = noteColor(note.color).tone;
  const [grow, setGrow] = useState(0);
  const progress = useProgress();
  const children = progress.children(note.id);
  const tasks = progress.of(note);
  const taskText = progressLabel(note, progress);
  const state = `${selected ? ' selected' : ''}${dropTarget ? ' drop-target' : ''}${note.canvas.done ? ' done' : ''}${project ? ' project' : ''}${fanQuiet ? ' fan-quiet' : ''}${grow > 0 ? ' is-expanded' : ''}${dimmed ? ' is-dimmed' : ''}${peeking ? ' is-peeking' : ''}`;
  return <>
    <article className={`canvas-note${state}`} style={{ left: position.x, top: position.y, '--card-tone': tone, '--preview-extra': `${grow}px` } as CSSProperties}
      // biome-ignore lint/a11y/noNoninteractiveTabindex: 카드는 키보드로 선택·열기(Enter)·삭제하는 캔버스 항목이라 포커스를 받아야 한다.
      data-note-id={note.id} onPointerDown={(event) => actions.pointerDown(event, note.id)} onPointerLeave={() => actions.pointerLeave(note.id)} onDoubleClick={() => actions.open(note)} onContextMenu={(event) => actions.contextMenu(event, note.id)} tabIndex={0} aria-label={`${title} 카드`}>
      <CardHeader title={title} marker={preview.title} project={project} done={note.canvas.done} onEnter={() => actions.open(note)} onToggleDone={() => actions.toggleDone(note.id)}/>
      <CardPreview id={`card-preview-${note.id}`} label={title} rows={preview.rows} empty={!note.text.trim()} onGrow={setGrow}/>
      {project ? <ProjectSummary label={title} count={children.length} progress={tasks} text={taskText} peek={peeking === undefined ? undefined : { open: peeking, onToggle: () => actions.togglePeek(note.id), targetId: `card-fan-${note.id}` }}/>
        : <div className="canvas-card-meta">{tasks.total > 0 && <small title={taskText.title}>할 일 {taskText.short}</small>}</div>}
      {note.canvas.next.length > 0 && <LinkHandle title={title} count={note.canvas.next.length} onConnectStart={() => actions.connectStart(note.id)}/>}
    </article>
    {project && <ProjectFan id={`card-fan-${note.id}`} fan={projectFan(children)} position={position} tone={tone}/>}
  </>;
});
