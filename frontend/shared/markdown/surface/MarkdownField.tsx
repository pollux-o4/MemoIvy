import { useRef } from 'react';
import type { EditorView } from '@codemirror/view';
import type { Draft } from '../../draft/useDraft';
import { MarkdownSurface } from './MarkdownSurface';

const NO_ASSETS: Record<string, string> = {};

/**
 * 초안을 메모 편집기 표면에 잇는 바인딩. 목록 이어쓰기·체크박스·서식 단축키가 메모와 같고, 저장 시점은 초안 규칙을 따른다.
 * Esc는 되돌리고 Ctrl/⌘+Enter는 편집을 마친다.
 */
export function MarkdownField({ draft, label, placeholder, className, onExternal }: {
  draft: Draft; label: string; placeholder?: string; className?: string; onExternal: (url: string) => void;
}) {
  const view = useRef<EditorView | null>(null);
  const current = () => view.current?.state.doc.toString() ?? draft.value;
  return <div className={className} role="group"
    // 편집기보다 먼저 받아 Mod-Enter(빈 줄 삽입) 대신 편집을 마친다.
    onKeyDownCapture={(event) => {
      if (event.nativeEvent.isComposing) return;
      const revert = event.key === 'Escape';
      if (!revert && !(event.key === 'Enter' && (event.ctrlKey || event.metaKey))) return;
      event.preventDefault(); event.stopPropagation();
      if (revert) draft.revert();
      view.current?.contentDOM.blur();
    }}
    // 입력 중 캔버스 단축키가 동작하지 않게 한다.
    onKeyDown={(event) => event.stopPropagation()}
    onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) draft.finish(current()); }}>
    <MarkdownSurface value={draft.value} label={label} placeholder={placeholder} compact raw={false} assetUrls={NO_ASSETS}
      onChange={draft.change}
      onComposing={(active) => draft.compose(active, active ? undefined : current())}
      onReady={(editor) => { view.current = editor; }}
      onExternal={onExternal}
      onViewImage={() => {}}/>
  </div>;
}
