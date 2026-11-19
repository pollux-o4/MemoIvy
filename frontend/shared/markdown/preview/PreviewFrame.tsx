import type { ReactNode, SyntheticEvent } from 'react';
import { Disclosure } from '../../Disclosure';
import { useCollapsible } from '../../useCollapsible';
import type { PreviewItem } from './model';

export function usePreviewDisclosure(rows: readonly PreviewItem[], collapsedRows: number) {
  return useCollapsible<HTMLUListElement>((list) => {
    const last = list.children[Math.min(collapsedRows, list.children.length) - 1] as HTMLElement | undefined;
    const height = last ? last.offsetTop - list.offsetTop + last.offsetHeight : 0;
    list.parentElement?.style.setProperty('--preview-trigger-offset', `${height}px`);
    return height;
  }, [rows, collapsedRows]);
}

const stop = (event: SyntheticEvent) => event.stopPropagation();

export function PreviewFrame({ label, targetId, preview, children }: {
  label: string; targetId: string; children: ReactNode; preview: Pick<ReturnType<typeof usePreviewDisclosure>, 'needsReveal' | 'expanded' | 'toggle'>;
}) {
  return <div className={`preview-frame${preview.needsReveal ? ' has-disclosure' : ''}${preview.expanded ? ' is-open' : ''}`}>
    {/* biome-ignore lint/a11y/noStaticElementInteractions: 버튼의 포인터 입력이 카드 끌기나 열기로 번지는 것만 막는다. */}
    {preview.needsReveal && <div className="preview-disclosure" onPointerDown={stop} onDoubleClick={stop}>
      <Disclosure expanded={preview.expanded} onToggle={preview.toggle} targetId={targetId} label={`${label} 본문`}/>
    </div>}
    {children}
  </div>;
}
