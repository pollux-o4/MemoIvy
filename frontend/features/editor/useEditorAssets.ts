import { useRef } from 'react';
import type { MemoStore } from '../../shared/useMemoivy';
import { useAssetUrls } from '../../shared/useAssetUrls';
import type { EditorView } from '@codemirror/view';
import { imageMarkup } from '../../shared/markdown/surface/image_tools/commands';
import { parseListItem } from '../../shared/markdown/list_item';

export function useEditorAssets(store: MemoStore, change: (value: string) => void, currentDraft: () => string) {
  const assetUrls = useAssetUrls(store);
  const editorView = useRef<EditorView | null>(null);

  const addFile = async (file: File, setError: (error: string | null) => void, drop?: { x: number; y: number }) => {
    if (file.size > 20 * 1024 * 1024) { setError('파일은 20 MiB 이하만 첨부할 수 있어요.'); return; }
    try {
      const dataUrl = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = () => reject(new Error('파일을 읽지 못했습니다.'));
        reader.readAsDataURL(file);
      });
      const result = await store.request('importAsset', { name: file.name, mime: file.type || 'application/octet-stream', data: dataUrl.split(',')[1] });
      if (!result.asset) return;
      const view = editorView.current;
      const line = view?.state.doc.lineAt(view.state.selection.main.head);
      const body = imageMarkup(file.name.replace(/[\]|\r\n]/g, ''), undefined, result.asset.id);
      const dropPosition = drop && view ? view.posAtCoords({ x: view.contentDOM.getBoundingClientRect().left, y: drop.y }, false) : null;
      const targetLine = view && dropPosition !== null ? view.state.doc.lineAt(dropPosition) : line;
      if (view && targetLine) {
        const item = parseListItem(targetLine.text);
        const indent = ' '.repeat(item?.contentIndent ?? 0);
        const before = targetLine.text.trim() ? '\n' : '';
        const insert = `${before}${indent}${body}\n${indent}`;
        const from = before ? targetLine.to : targetLine.from;
        view.dispatch({
          changes: { from, to: targetLine.to, insert },
          selection: { anchor: from + insert.length },
          scrollIntoView: true,
          userEvent: 'input.paste',
        });
        view.focus();
      } else change(`${currentDraft()}${currentDraft() ? '\n\n' : ''}${body}\n`);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  return { assetUrls, editorView, addFile };
}
