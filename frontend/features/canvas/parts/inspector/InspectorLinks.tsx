import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { canvasTitle } from '../../model/labels';
import { LinkPicker } from './LinkPicker';

/** 3층 · 흐름: 이전 → 이 메모 → 다음. 연결이 없으면 추가 칸만 보인다. 신호는 다음 메모가 있을 때만. */
export function InspectorLinks({ note, notes, onSelect, onToggleLink, onSignal }: {
  note: Note; notes: Note[]; onSelect: (id: string) => void; onToggleLink: (note: Note, id: string) => void; onSignal: () => void;
}) {
  const incoming = notes.filter((item) => item.canvas.next.includes(note.id));
  const outgoing = note.canvas.next.flatMap((id) => notes.find((item) => item.id === id) ?? []);
  const candidates = notes.filter((item) => item.id !== note.id && !note.canvas.next.includes(item.id));
  return <section className="inspector-group" aria-label="연결">
    <header><b>연결</b>{outgoing.length > 0 && <button className="inspector-mini" title="연결된 다음 메모로 신호를 보냅니다. 완료 상태는 바꾸지 않습니다." onClick={onSignal}><Icon name="signal" size={13}/>신호</button>}</header>
    {incoming.length > 0 && <div className="inspector-chips" role="group" aria-label="이전 메모"><small>이전</small>{incoming.map((item) =>
      <button className="inspector-chip" key={item.id} title={canvasTitle(item)} onClick={() => onSelect(item.id)}>{canvasTitle(item)}</button>)}</div>}
    {outgoing.length > 0 && <div className="inspector-chips" role="group" aria-label="다음 메모"><small>다음</small>{outgoing.map((item) =>
      <span className="inspector-chip removable" key={item.id}>
        <button title={canvasTitle(item)} onClick={() => onSelect(item.id)}>{canvasTitle(item)}</button>
        <button className="inspector-disconnect" aria-label={`${canvasTitle(item)} 연결 해제`} title="이 연결 해제 · 메모는 유지" onClick={() => onToggleLink(note, item.id)}><Icon name="close" size={11}/></button>
      </span>)}</div>}
    {candidates.length > 0 && <LinkPicker candidates={candidates} onPick={(id) => onToggleLink(note, id)}/>}
  </section>;
}
