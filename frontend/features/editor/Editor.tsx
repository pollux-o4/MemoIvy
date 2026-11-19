import { type CSSProperties, useEffect, useState } from 'react';
import type { EditorView } from '@codemirror/view';
import { Icon } from '../../shared/Icon';
import { noteColor } from '../../shared/model';
import { useSaveLabel } from '../../shared/useSaveLabel';
import type { MemoStore } from '../../shared/useMemoivy';
import { ConfirmDialog } from '../../shared/ConfirmDialog';
import { ImageViewer } from '../../shared/ImageViewer';
import { EditorContent } from './EditorContent';
import { EditorToolbar } from './EditorToolbar';
import { MarkdownSurface } from '../../shared/markdown/surface/MarkdownSurface';
import { useEditorAssets } from './useEditorAssets';
import { useNoteDraft } from './useNoteDraft';

export function Editor({ store, noteId, onClose }: { store: MemoStore; noteId: string; onClose: () => void }) {
  const note = store.notes.find((item) => item.id === noteId);
  const draft = useNoteDraft(store, note);
  const [raw, setRaw] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [viewingImage, setViewingImage] = useState<string | null>(null);
  const { assetUrls, editorView, addFile } = useEditorAssets(store, draft.change, () => draft.latest.current);
  const saveLabel = useSaveLabel(store.saving || draft.dirty);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || !(event.ctrlKey || event.metaKey)) return;
      if (event.shiftKey && event.key.toLowerCase() === 'm') { event.preventDefault(); setRaw((mode) => !mode); }
      if (!event.shiftKey && event.key.toLowerCase() === 'w') {
        event.preventDefault();
        void draft.save().then((saved) => { if (saved) onClose(); });
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [draft.save, onClose]);

  const changeColor = (color: string) => note && void store.updateNote(note, undefined, undefined, color).catch((reason) => draft.setError(String(reason)));
  const deleteNote = async () => {
    if (!note) return;
    try { await store.removeNote(note.id); setDeleteConfirm(false); onClose(); }
    catch (reason) { draft.setError(reason instanceof Error ? reason.message : String(reason)); setDeleteConfirm(false); }
  };
  const hasLinks = note && (note.canvas.next.length > 0 || store.notes.some((item) => item.canvas.next.includes(note.id)));

  if (!note) return <div className="editor-screen"><div className="editor-topbar"><button className="icon-button" onClick={onClose}><Icon name="close"/></button><span>메모를 찾을 수 없습니다</span></div></div>;

  return <div className={`editor-screen note-${note.color}`} style={{ '--markdown-code-tint': noteColor(note.color).accent } as CSSProperties}>
    <EditorToolbar note={note} store={store} onClose={onClose} raw={raw} onToggleRaw={() => setRaw(!raw)} onDelete={() => setDeleteConfirm(true)} onColor={changeColor}/>
    <EditorContent onFile={(file, drop) => void addFile(file, draft.setError, drop)}>
      <MarkdownSurface
        value={draft.draft}
        onChange={draft.change}
        raw={raw}
        assetUrls={assetUrls}
        onReady={(view: EditorView | null) => {
          editorView.current = view;
          if (view) {
            view.dispatch({ selection: { anchor: view.state.doc.length }, scrollIntoView: false });
            view.focus();
          }
        }}
        onComposing={draft.updateComposing}
        onExternal={(url) => { void store.bridge.openExternal(url); }}
        onViewImage={setViewingImage}
      />
    </EditorContent>
    <footer className={`editor-status${draft.error ? ' save-error' : ''}`}>
      {draft.error ? <>
        <span>{draft.error}</span>
        {draft.conflict ? <>
          <button onClick={() => void draft.overwriteDraft()}>내 초안으로 덮어쓰기</button>
          <button onClick={draft.useRemoteVersion}>최신 버전 불러오기</button>
        </> : <button onClick={() => void draft.save()}>다시 시도</button>}
      </> : <span key={saveLabel} className="save-label">{saveLabel}</span>}
    </footer>
    {viewingImage && assetUrls[viewingImage] && <ImageViewer src={assetUrls[viewingImage]} alt={store.assets.find((asset) => asset.id === viewingImage)?.name ?? '이미지'} onClose={() => setViewingImage(null)}/>}
    {deleteConfirm && <ConfirmDialog
      question="이 메모를 삭제할까요?"
      detail={hasLinks ? '삭제한 메모는 복구할 수 없습니다. 내용과 연결이 모두 사라져요.' : '삭제한 메모는 복구할 수 없습니다.'}
      acceptLabel="삭제"
      onCancel={() => setDeleteConfirm(false)}
      onAccept={() => void deleteNote()}
    />}
  </div>;
}
