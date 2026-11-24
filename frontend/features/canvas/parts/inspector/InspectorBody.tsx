import { useEffect, useMemo, useRef, useState } from 'react';
import type { Draft } from '../../../../shared/draft/useDraft';
import { markdownPreview } from '../../../../shared/markdown/preview/model';
import { PreviewList } from '../../../../shared/markdown/preview/PreviewList';
import { MarkdownField } from '../../../../shared/markdown/surface/MarkdownField';

const PREVIEW_ROWS = 4;

/**
 * 인스펙터 본문: 메모·프로젝트 모두 같은 미리보기로 보인다.
 * 소개를 고칠 수 있으면(프로젝트) 미리보기를 누르거나 Enter로 편집기로 바뀌고, 포커스가 나가면 미리보기로 돌아온다. 비어 있으면 바로 편집기를 보이고, 포커스가 들어온 동안은 입력으로 비지 않게 되어도 편집기를 유지한다.
 */
export function InspectorBody({ text, edit, onToggleTask }: {
  text: string;
  edit?: { draft: Draft; label: string; placeholder: string; onExternal: (url: string) => void };
  onToggleTask: (sourceLine: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const field = useRef<HTMLDivElement>(null);
  const showEditor = Boolean(edit && (editing || !edit.draft.value.trim()));
  // 편집 중에는 미리보기를 그리지 않으므로 입력마다 다시 파싱하지 않는다.
  const rows = useMemo(() => showEditor ? [] : markdownPreview(text, PREVIEW_ROWS).rows, [showEditor, text]);
  // 편집기가 마운트를 마친 다음 프레임에 포커스를 준다(개발 StrictMode의 재생성 중 포커스를 잃지 않게).
  useEffect(() => {
    if (!editing) return;
    const frame = requestAnimationFrame(() => field.current?.querySelector<HTMLElement>('.cm-content')?.focus());
    return () => cancelAnimationFrame(frame);
  }, [editing]);

  if (edit && showEditor) {
    return <div ref={field} className="inspector-body editing" onFocus={() => setEditing(true)} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setEditing(false); }}>
      <MarkdownField className="inspector-intro" draft={edit.draft} label={edit.label} placeholder={edit.placeholder} onExternal={edit.onExternal}/>
    </div>;
  }
  if (!rows.length) return null;
  const list = <PreviewList className="inspector-preview" rows={rows} onToggleTask={onToggleTask}/>;
  if (!edit) return list;
  return <div className="inspector-body" role="group" aria-label={`${edit.label} · 눌러서 고치기`} tabIndex={0}
    onClick={(event) => { if (!(event.target as HTMLElement).closest('button, a, input')) setEditing(true); }}
    onKeyDown={(event) => { if (event.target === event.currentTarget && event.key === 'Enter') { event.preventDefault(); event.stopPropagation(); setEditing(true); } }}>
    {list}
  </div>;
}
