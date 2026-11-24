import { useEffect, useMemo, useRef, useState } from 'react';
import { Icon } from '../../../../shared/Icon';
import type { Draft } from '../../../../shared/draft/useDraft';
import { markdownPreview } from '../../../../shared/markdown/preview/model';
import { PreviewList } from '../../../../shared/markdown/preview/PreviewList';
import { MarkdownField } from '../../../../shared/markdown/surface/MarkdownField';

const PREVIEW_ROWS = 4;

/**
 * 인스펙터 본문: 메모·프로젝트 모두 같은 미리보기로 보인다.
 * 소개를 고칠 수 있으면(프로젝트) 고치기 버튼이나 미리보기를 눌러 편집기로 바뀌고, 포커스가 나가면 미리보기로 돌아온다. 비어 있으면 바로 편집기를 보이고, 포커스가 들어온 동안은 입력으로 비지 않게 되어도 편집기를 유지한다.
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
    return <div ref={field} className="inspector-body editing" role="group" aria-label={`${edit.label} 편집`} onFocus={() => setEditing(true)} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setEditing(false); }}>
      <MarkdownField className="inspector-intro" draft={edit.draft} label={edit.label} placeholder={edit.placeholder} onExternal={edit.onExternal}/>
    </div>;
  }
  if (!rows.length) return null;
  const list = <PreviewList className="inspector-preview" rows={rows} onToggleTask={onToggleTask}/>;
  if (!edit) return list;
  // 키보드·보조기기는 '고치기' 버튼으로, 마우스는 미리보기 빈 곳을 눌러도 같은 편집기로 들어간다.
  return <div className="inspector-body" role="group" aria-label={edit.label}
    onPointerUp={(event) => { if (event.button === 0 && !(event.target as HTMLElement).closest('button, a, input')) setEditing(true); }}>
    {list}
    <button type="button" className="inspector-body-edit" aria-label={`${edit.label} 고치기`} title={`${edit.label} 고치기`} onClick={() => setEditing(true)}><Icon name="source" size={13}/></button>
  </div>;
}
