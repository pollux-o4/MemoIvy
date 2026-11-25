import { type RefObject, useState } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { NOTE_COLORS, noteColor } from '../../../../shared/model';
import { canvasTitle } from '../../model/labels';
import { bindInput } from '../../../../shared/draft/inputBinding';
import { InspectorBody } from './InspectorBody';
import { useTitledText } from './useTitledText';

/**
 * 1층 · 정체성: 색·이름·(프로젝트면) 소개를 그 자리에서 고친다. 본문은 InspectorBody가 같은 미리보기로 그린다.
 */
export function InspectorIdentity({ note, revision, titleRef, onSaveText, onToggleTask, onColor, onClose, onExternal }: {
  note: Note; revision?: number; titleRef: RefObject<HTMLInputElement | null>;
  onSaveText: (text: string, baseRevision: number | undefined, origin: string) => Promise<number | undefined>; onColor: (color: string) => void; onClose: () => void;
  onToggleTask: (sourceLine: number) => void;
  onExternal: (url: string) => void;
}) {
  const project = Boolean(note.canvas.project);
  const { title, body: intro } = useTitledText(note.text, onSaveText, { version: revision, fallbackTitle: canvasTitle(note) });
  const [palette, setPalette] = useState(false);
  return <section className="inspector-identity" aria-label={project ? '프로젝트 정보' : '메모 정보'}>
    <div className="inspector-title-row">
      <button className="inspector-color" style={{ background: noteColor(note.color).accent }} aria-label={`색 바꾸기 · 지금 ${noteColor(note.color).label}`} aria-expanded={palette} title="색 바꾸기" onClick={() => setPalette(!palette)}/>
      <input ref={titleRef} className="inspector-title" aria-label={project ? '프로젝트 이름' : '메모 제목'} placeholder={project ? '프로젝트 이름' : '제목'} title="이름 바꾸기 · F2" {...bindInput(title)}/>
      <button className="inspector-head-action" aria-label="선택 해제" title="선택 해제 · Esc" onClick={onClose}><Icon name="close" size={14}/></button>
    </div>
    {palette && <div className="inspector-palette" role="radiogroup" aria-label="메모 색">
      {NOTE_COLORS.map((color) => <button key={color} role="radio" aria-checked={note.color === color} aria-label={noteColor(color).label} title={noteColor(color).label} style={{ background: noteColor(color).accent }} onClick={() => { onColor(color); setPalette(false); }}/>)}
    </div>}
    <InspectorBody text={note.text} onToggleTask={onToggleTask}
      edit={project ? { draft: intro, label: '프로젝트 소개', placeholder: '이 프로젝트를 한두 줄로 소개해 보세요', onExternal } : undefined}/>
  </section>;
}
