import { childCountLabel } from '../../../../shared/hierarchy';
import type { Note } from '../../../../shared/engine_contract';
import { Icon, type IconName } from '../../../../shared/Icon';

/** 2층 · 상태와 주 동작: 한눈에 보는 상태 한 줄 + 가장 흔한 동작 하나(들어가기·열기), 프로젝트는 아래 행에 보조 '메모로 열기'. 완료는 상태 옆 토글. */
export function InspectorPrimary({ note, childCount, tasks, onOpen, onOpenAsNote, onToggleDone }: {
  note: Note; childCount: number; tasks?: { short: string; title: string };
  onOpen: () => void; onOpenAsNote: () => void; onToggleDone: () => void;
}) {
  const project = Boolean(note.canvas.project);
  const facts = [project && childCountLabel(childCount), tasks && `${project ? '완료' : '할 일'} ${tasks.short}`].filter(Boolean).join(' · ');
  return <section className="inspector-primary">
    <div className="inspector-status">
      <span title={tasks?.title}>{facts || (project ? '빈 프로젝트' : '할 일 없음')}</span>
      <button className="inspector-done" aria-pressed={note.canvas.done} title="완료 상태 바꾸기 · Ctrl+Enter" onClick={onToggleDone}>
        <Icon name="check" size={13}/>{note.canvas.done ? '완료됨' : '완료'}
      </button>
    </div>
    <div className="inspector-actions-stack">
      <InspectorAction primary icon={project ? 'enter' : 'note'} label={project ? '프로젝트 들어가기' : '메모 열기'} keys="Enter" shortcut="Enter" hint="두 번 누르기 또는 Enter" onClick={onOpen}/>
      {project && <InspectorAction icon="note" label="메모로 열기" keys="⇧ Enter" shortcut="Shift+Enter" hint="Shift+Enter" onClick={onOpenAsNote}/>}
    </div>
  </section>;
}

/** 인스펙터 동작 한 줄: 아이콘·이름·단축키 배치는 같고 강조(primary)만 다르다. 기본 모양은 정리 줄과 같은 .inspector-action. */
function InspectorAction({ primary = false, icon, label, keys, shortcut, hint, onClick }: {
  primary?: boolean; icon: IconName; label: string; keys: string; shortcut: string; hint: string; onClick: () => void;
}) {
  return <button className={primary ? 'inspector-action primary' : 'inspector-action'} aria-keyshortcuts={shortcut} title={`${label} · ${hint}`} onClick={onClick}>
    <Icon name={icon} size={15}/><span>{label}</span><kbd className="inspector-kbd" aria-hidden>{keys}</kbd>
  </button>;
}
