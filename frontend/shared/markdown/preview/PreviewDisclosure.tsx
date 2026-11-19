import type { SyntheticEvent } from 'react';
import { Disclosure } from '../../Disclosure';
import { useCollapsible } from '../../useCollapsible';
import type { PreviewItem } from './model';

export function usePreviewDisclosure(rows: readonly PreviewItem[], collapsedRows: number) {
  return useCollapsible<HTMLUListElement>((list) => {
    const last = list.children[Math.min(collapsedRows, list.children.length) - 1] as HTMLElement | undefined;
    return last ? last.offsetTop - list.offsetTop + last.offsetHeight : 0;
  }, [rows, collapsedRows]);
}

const stop = (event: SyntheticEvent) => event.stopPropagation();

export function PreviewDisclosure({ label, targetId, preview }: {
  label: string; targetId: string; preview: Pick<ReturnType<typeof usePreviewDisclosure>, 'needsReveal' | 'expanded' | 'toggle'>;
}) {
  if (!preview.needsReveal) return null;
  // biome-ignore lint/a11y/noStaticElementInteractions: 버튼의 포인터 입력이 카드 끌기나 열기로 번지는 것만 막는다.
  return <div className="preview-disclosure" onPointerDown={stop} onDoubleClick={stop}>
    <Disclosure expanded={preview.expanded} onToggle={preview.toggle} targetId={targetId} label={`${label} 본문`}/>
  </div>;
}
