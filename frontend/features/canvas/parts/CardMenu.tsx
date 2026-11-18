import './card_menu.css';
import type { Note } from '../../../shared/engine_contract';
import { Icon } from '../../../shared/Icon';
import { useMenu } from '../../../shared/menu/useMenu';
import { NOTE_COLORS, noteColor } from '../../../shared/model';

/** 메뉴 크기의 단일 출처: CSS 너비와 화면 끝 당김 계산이 함께 쓴다. */
export const CARD_MENU_SIZE = { width: 236, height: 300 };

export type CardMenuActions = {
  open: () => void; openAsNote: () => void; rename: () => void; color: (color: string) => void; toggleDone: () => void;
  copy: () => void; duplicate: () => void; remove: () => void;
};

/**
 * 카드 오른쪽 클릭(또는 Shift+F10·메뉴 키) 메뉴. 한 장이면 열기·이름 바꾸기까지, 여러 장이면 묶음 동작만.
 * 항목 순서: 주 동작 → 편집 → 색 → 상태 → 복사 → 위험. 단축키 힌트로 다음엔 키보드로 하게 돕는다.
 */
export function CardMenu({ at, notes, actions, onClose }: { at: { x: number; y: number }; notes: Note[]; actions: CardMenuActions; onClose: () => void }) {
  const { root } = useMenu(true, onClose);
  const single = notes.length === 1 ? notes[0] : undefined;
  const project = Boolean(single?.canvas.project);
  const allDone = notes.every((note) => note.canvas.done);
  const run = (action: () => void) => () => { onClose(); action(); };
  const row = (icon: Parameters<typeof Icon>[0]['name'], label: string, hint: string, action: () => void, destructive = false) =>
    <button className={`menu-row${destructive ? ' destructive' : ''}`} role="menuitem" onClick={run(action)}><Icon name={icon} size={15}/><span>{label}</span><small className="menu-hint">{hint}</small></button>;
  return <div ref={root} className="menu-surface card-menu" role="menu" aria-label={single ? '카드 메뉴' : `카드 ${notes.length}개 메뉴`} style={{ left: at.x, top: at.y, '--card-menu-width': `${CARD_MENU_SIZE.width}px` } as React.CSSProperties}
    onContextMenu={(event) => event.preventDefault()} onPointerDown={(event) => event.stopPropagation()}>
    {single && row(project ? 'enter' : 'note', project ? '프로젝트 들어가기' : '메모 열기', 'Enter', actions.open)}
    {single && project && row('note', '메모로 열기', 'Shift+Enter', actions.openAsNote)}
    {single && row('source', project ? '이름·소개 편집' : '제목 바꾸기', 'F2', actions.rename)}
    <div className="card-menu-colors" role="group" aria-label="색">
      {NOTE_COLORS.map((color) => <button key={color} role="menuitemradio" aria-checked={notes.every((note) => note.color === color)} aria-label={noteColor(color).label} title={noteColor(color).label} style={{ background: noteColor(color).accent }} onClick={run(() => actions.color(color))}/>)}
    </div>
    <hr/>
    {row('check', allDone ? '완료 취소' : '완료로 표시', 'Ctrl+Enter', actions.toggleDone)}
    {row('copy', '복사', 'Ctrl+C', actions.copy)}
    {row('add', '복제', 'Ctrl+D', actions.duplicate)}
    <hr/>
    {row('trash', notes.length > 1 ? `${notes.length}개 삭제…` : '삭제…', 'Delete', actions.remove, true)}
  </div>;
}
