import { ConfirmDialog } from '../../../shared/ConfirmDialog';
import type { Note } from '../../../shared/engine_contract';
import { canvasTitle } from '../model/labels';

/** 캔버스 메모 삭제 확인. 연결이 함께 사라지면 그 사실을 알린다. */
export function CanvasDeleteDialog({ ids, notes, onCancel, onAccept }: { ids: string[]; notes: Note[]; onCancel: () => void; onAccept: () => void }) {
  const hasLinks = ids.length > 1 || notes.some((note) => ids.includes(note.id) && note.canvas.next.length > 0 || note.canvas.next.some((target) => ids.includes(target)));
  const subject = ids.length === 1 ? notes.find((note) => note.id === ids[0]) : undefined;
  return <ConfirmDialog
    question={ids.length === 1 ? '이 메모를 삭제할까요?' : `메모 ${ids.length}개를 삭제할까요?`}
    subject={subject ? canvasTitle(subject) : undefined}
    detail={hasLinks ? '내용과 연결이 함께 지워져요. 이 창에서 Ctrl+Z로 되돌릴 수 있어요.' : '이 창에서 Ctrl+Z로 되돌릴 수 있어요.'}
    acceptLabel="삭제" onCancel={onCancel} onAccept={onAccept}/>;
}
