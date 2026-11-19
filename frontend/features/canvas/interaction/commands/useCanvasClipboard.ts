import { useRef } from 'react';
import type { Note } from '../../../shared/engine_contract';
import type { MemoStore } from '../../../shared/useMemoivy';
import { type CanvasPoint, savedPoint } from '../model/align';
import { type ClipPayload, copyNotes, pastePositions, plainText } from '../model/clipboard';

async function readSystemText(): Promise<string> {
  try { return await navigator.clipboard.readText(); } catch { return ''; }
}
function writeSystemText(text: string) {
  navigator.clipboard?.writeText(text).catch(() => undefined);
}

/**
 * 캔버스 클립보드: 복사(Ctrl+C)·붙여넣기(Ctrl+V)·복제(Ctrl+D).
 * 카드 묶음은 앱 안에 담고, 같은 내용을 평문으로 시스템 클립보드에도 써 다른 앱에서 쓸 수 있게 한다.
 * 시스템 클립보드가 그 뒤 바뀌었으면(다른 앱에서 복사) 그 글로 새 메모를 만든다.
 * shortcut: 프로젝트는 빈 프로젝트로만 복사(하위 메모 제외)하고 붙여넣기는 실행취소 기록에 없음, 깊은 복사·생성 취소가 필요해지면 확장.
 */
export function useCanvasClipboard({ store, notes, positions, selected, activeScope, onPasted }: {
  store: MemoStore; notes: Note[]; positions: Record<string, CanvasPoint>; selected: string[]; activeScope: string | null;
  onPasted: (ids: string[]) => void;
}) {
  const clip = useRef<{ payload: ClipPayload; text: string; round: number } | null>(null);

  const place = async (payload: ClipPayload, round: number) => {
    const spots = pastePositions(payload, round);
    const created: Note[] = [];
    for (const item of payload.items) {
      const note = await store.createNote({ text: item.text, color: item.color, project: item.project, parent: activeScope });
      if (!note) break;
      created.push(note);
    }
    if (!created.length) return;
    await store.mutateCanvas(created.map((note, index) => ({
      id: note.id,
      canvas: { position: savedPoint(spots[index]), done: payload.items[index].done, next: payload.items[index].next.flatMap((target) => created[target] ? [created[target].id] : []) },
    })));
    onPasted(created.map((note) => note.id));
  };

  const copy = () => {
    const payload = copyNotes(selected, notes, positions);
    if (!payload) return false;
    const text = plainText(payload);
    clip.current = { payload, text, round: 0 };
    writeSystemText(text);
    return true;
  };

  const paste = async () => {
    const system = await readSystemText();
    const own = clip.current;
    if (own && (!system || system === own.text)) {
      own.round += 1;
      await place(own.payload, own.round);
      return;
    }
    if (!system.trim()) return;
    const note = await store.createNote({ text: system, parent: activeScope });
    if (note) onPasted([note.id]);
  };

  const duplicate = async () => {
    const payload = copyNotes(selected, notes, positions);
    if (payload) await place(payload, 1);
  };

  return { copy, paste, duplicate };
}
