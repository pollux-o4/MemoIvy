import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Compartment, EditorState } from '@codemirror/state';
import { indentWithTab, history, historyKeymap, defaultKeymap } from '@codemirror/commands';
import { EditorView, keymap, drawSelection, Decoration, ViewPlugin, WidgetType, type ViewUpdate } from '@codemirror/view';
import { markdown } from '@codemirror/lang-markdown';
import { defaultHighlightStyle, syntaxHighlighting, syntaxTree } from '@codemirror/language';
import { Icon } from '../../shared/Icon';
import { titleOf } from '../../shared/model';
import { NOTE_PALETTE } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';
import type { Note } from '../../shared/engine_contract';

const editorTheme = EditorView.theme({
  '&': { height: '100%', fontSize: '13.333px', color: 'var(--note-text)' },
  '.cm-scroller': { fontFamily: 'inherit', lineHeight: '1.5', overflow: 'auto', padding: '20px 0 24px' },
  '.cm-content': { maxWidth: 'none', margin: '0', padding: '0 20px', caretColor: 'var(--memoivy-accent)' },
  '.cm-line': { padding: '0' },
  '.cm-cursor': { borderLeftColor: 'var(--memoivy-accent)' },
  '.cm-selectionBackground, ::selection': { background: 'rgba(10, 108, 255, .28) !important' },
  '.cm-activeLine': { backgroundColor: 'transparent' },
  '.cm-gutters': { display: 'none' },
  '.cm-focused': { outline: 'none' },
  '.cm-heading': { fontWeight: '680', letterSpacing: '-.035em', color: 'var(--ink)' },
  '.cm-strong': { fontWeight: '700' },
  '.cm-emphasis': { fontStyle: 'italic' },
  '.cm-link': { color: 'var(--accent)', textDecoration: 'underline', textUnderlineOffset: '3px' },
  '.cm-mark-hidden': { display: 'none' },
});

class AssetWidget extends WidgetType {
  constructor(readonly description: string, readonly src: string | undefined, readonly width?: number) { super(); }
  eq(other: AssetWidget) { return this.description === other.description && this.src === other.src && this.width === other.width; }
  toDOM() {
    const wrapper = document.createElement('span'); wrapper.className = 'cm-asset-preview';
    if (this.src) { const image = document.createElement('img'); image.src = this.src; image.alt = this.description || '이미지'; image.draggable = false; image.style.width = this.width ? `${Math.min(4096, Math.max(64, this.width))}px` : 'auto'; wrapper.append(image); }
    else { const placeholder = document.createElement('div'); placeholder.textContent = '이미지를 찾을 수 없음'; placeholder.setAttribute('role', 'img'); placeholder.setAttribute('aria-label', this.description || '이미지를 찾을 수 없음'); wrapper.append(placeholder); }
    return wrapper;
  }
}

class CheckboxWidget extends WidgetType {
  constructor(readonly state: ' ' | 'x' | '/', readonly from: number) { super(); }
  eq(other: WidgetType) { return other instanceof CheckboxWidget && this.state === other.state && this.from === other.from; }
  toDOM(view: EditorView) {
    const box = document.createElement('input'); box.type = 'checkbox'; box.className = 'cm-task-checkbox'; box.checked = this.state === 'x';
    box.indeterminate = this.state === '/'; box.setAttribute('aria-label', this.state === 'x' ? '할 일 완료됨' : this.state === '/' ? '진행 중인 할 일' : '할 일 완료 표시');
    box.addEventListener('click', (event) => {
      event.preventDefault(); if (view.composing) return;
      const next = event.shiftKey ? this.state === '/' ? ' ' : '/' : this.state === 'x' ? ' ' : 'x';
      const from = this.from; const to = from + 3;
      view.dispatch({ changes: { from, to, insert: `[${next}]` }, selection: { anchor: Math.min(view.state.selection.main.head, view.state.doc.length) }, scrollIntoView: false }); view.focus();
    });
    return box;
  }
}

function liveMarkdown(assetUrls: Record<string, string>) {
  const hiddenMark = Decoration.replace({ class: 'cm-mark-hidden' });
  const activeLine = Decoration.line({ class: 'cm-activeLine' });
  return ViewPlugin.fromClass(class {
    decorations;
    constructor(view: EditorView) { this.decorations = this.build(view); }
    update(update: ViewUpdate) { if (!update.view.composing && (update.docChanged || update.selectionSet || update.viewportChanged)) this.decorations = this.build(update.view); }
    build(view: EditorView) {
      const active = view.state.doc.lineAt(view.state.selection.main.head).number;
      const activeLineObj = view.state.doc.line(active);
      const marks = new Set(['HeaderMark', 'EmphasisMark', 'StrongEmphasisMark', 'StrikethroughMark', 'LinkMark', 'ImageMark', 'CodeMark']);
      const ranges: Array<{ from: number; to: number; value: Decoration }> = [{ from: activeLineObj.from, to: activeLineObj.from, value: activeLine }];
      for (let lineNumber = 1; lineNumber <= view.state.doc.lines; lineNumber++) {
        const line = view.state.doc.line(lineNumber);
        const match = /^!\[([^\]|]*)(?:\|(\d+))?\]\(asset:([a-f\d]{64})\)$/i.exec(line.text.trim());
        if (match) { ranges.push({ from: line.from, to: line.to, value: Decoration.replace({ widget: new AssetWidget(match[1], assetUrls[match[3]], Number(match[2]) || undefined) }) }); continue; }
        if (line.number === active) continue;
        const checkbox = /^(\s*[-*+]\s+)(\[([ x/])\])(?=\s)/i.exec(line.text);
        if (checkbox) {
          const from = line.from + checkbox[1].length;
          const state = checkbox[3].toLowerCase() as ' ' | 'x' | '/';
          ranges.push({ from, to: from + checkbox[2].length, value: Decoration.replace({ widget: new CheckboxWidget(state, from) }) });
        }
      }
      syntaxTree(view.state).iterate({ from: view.visibleRanges[0]?.from ?? 0, to: view.visibleRanges.at(-1)?.to ?? view.state.doc.length, enter(node) {
        if (marks.has(node.name) && view.state.doc.lineAt(node.from).number !== active && node.to > node.from) ranges.push({ from: node.from, to: node.to, value: hiddenMark });
      } });
      return Decoration.set(ranges.sort((a, b) => a.from - b.from || a.to - b.to).map((item) => item.value.range(item.from, item.to)), true);
    }
  }, { decorations: (plugin) => plugin.decorations });
}

function formattingKeymap() {
  const wrap = (left: string, right = left) => (view: EditorView) => {
    if (view.composing) return false;
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to);
    const remove = selected.startsWith(left) && selected.endsWith(right) && selected.length >= left.length + right.length;
    const insert = remove ? selected.slice(left.length, selected.length - right.length) : `${left}${selected || '텍스트'}${right}`;
    const start = from + (remove ? 0 : left.length);
    view.dispatch({ changes: { from, to, insert }, selection: { anchor: start, head: start + (remove ? insert.length : selected ? selected.length : 2) }, scrollIntoView: true });
    view.focus(); return true;
  };
  const linePrefix = (prefix: string) => (view: EditorView) => {
    if (view.composing) return false;
    const line = view.state.doc.lineAt(view.state.selection.main.head);
    const text = line.text;
    const existing = /^(?:#{1,2}\s+|[-*+]\s+|\d+[.)]\s+|[-*+]\s+\[[ x/]]\s+)/.exec(text)?.[0] ?? '';
    const insert = text.startsWith(prefix) ? text.slice(prefix.length) : `${prefix}${text.slice(existing.length)}`;
    view.dispatch({ changes: { from: line.from, to: line.to, insert } }); view.focus(); return true;
  };
  const mark = (regexp: RegExp, replacement: (match: RegExpExecArray) => string) => (view: EditorView) => {
    if (view.composing) return false;
    const line = view.state.doc.lineAt(view.state.selection.main.head);
    const match = regexp.exec(line.text);
    if (!match) return false;
    const next = replacement(match);
    view.dispatch({ changes: { from: line.from, to: line.to, insert: next } }); view.focus(); return true;
  };
  return [
    { key: 'Mod-b', run: wrap('**') }, { key: 'Mod-i', run: wrap('*') }, { key: 'Mod-Shift-x', run: wrap('~~') },
    { key: 'Mod-1', run: linePrefix('# ') }, { key: 'Mod-2', run: linePrefix('## ') },
    { key: 'Mod-Shift-7', run: linePrefix('1. ') }, { key: 'Mod-Shift-8', run: linePrefix('- ') }, { key: 'Mod-Shift-9', run: linePrefix('- [ ] ') },
    { key: 'Mod-k', run: wrap('[', '](https://example.com)') },
    { key: 'Mod-Enter', run: mark(/^(\s*[-*+]\s+\[)([ x/])(\])/, (m) => `${m[1]}${m[2].toLowerCase() === 'x' ? ' ' : 'x'}${m[3]}`) },
  ];
}

function MarkdownSurface({ value, onChange, raw, assetUrls, onReady, onComposing, onExternal }: { value: string; onChange: (value: string) => void; raw: boolean; assetUrls: Record<string, string>; onReady: (view: EditorView | null) => void; onComposing: (value: boolean) => void; onExternal: (url: string) => void }) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const rawMode = useMemo(() => new Compartment(), []);
  const onChangeRef = useRef(onChange);
  const onReadyRef = useRef(onReady);
  const onComposingRef = useRef(onComposing);
  const onExternalRef = useRef(onExternal);
  onChangeRef.current = onChange;
  onReadyRef.current = onReady;
  onComposingRef.current = onComposing;
  onExternalRef.current = onExternal;
  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({ doc: value, extensions: [history(), drawSelection(), EditorView.lineWrapping, markdown(), syntaxHighlighting(defaultHighlightStyle), editorTheme, rawMode.of(raw ? [] : [liveMarkdown(assetUrls)]), keymap.of([...formattingKeymap(), ...historyKeymap, ...defaultKeymap, indentWithTab]), EditorView.domEventHandlers({ compositionstart: () => { onComposingRef.current(true); return false; }, compositionend: () => { window.setTimeout(() => onComposingRef.current(false), 0); return false; }, click: (event, currentView) => { if (!(event.ctrlKey || event.metaKey)) return false; const position = currentView.posAtCoords({ x: event.clientX, y: event.clientY }); if (position === null) return false; const line = currentView.state.doc.lineAt(position); const match = /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/.exec(line.text); if (!match) return false; onExternalRef.current(match[2]); return true; } }), EditorView.updateListener.of((update) => { if (update.docChanged) onChangeRef.current(update.state.doc.toString()); })] });
    const editor = new EditorView({ state, parent: host.current }); view.current = editor; onReadyRef.current(editor);
    return () => { onReadyRef.current(null); editor.destroy(); view.current = null; };
  }, []);
  useEffect(() => { view.current?.dispatch({ effects: rawMode.reconfigure(raw ? [] : [liveMarkdown(assetUrls)]) }); }, [raw, assetUrls, rawMode]);
  useEffect(() => {
    const editor = view.current;
    if (editor && value !== editor.state.doc.toString()) editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
  }, [value]);
  return <div className={`markdown-surface${raw ? ' raw-mode' : ''}`} ref={host}/>;
}

export function Editor({ store, noteId, onClose }: { store: MemoStore; noteId: string; onClose: () => void }) {
  const note = store.notes.find((item) => item.id === noteId);
  const [draft, setDraft] = useState(note?.text ?? '');
  const [raw, setRaw] = useState(false);
  const [menu, setMenu] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const composingRef = useRef(false);
  const [assetUrls, setAssetUrls] = useState<Record<string, string>>({});
  const editorView = useRef<EditorView | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const cancelDelete = useRef<HTMLButtonElement>(null);
  const objectUrls = useRef<string[]>([]);
  const latest = useRef(draft);
  const baseRevision = useRef<number | undefined>(undefined);
  const timer = useRef<number | undefined>(undefined);
  const started = useRef<number | undefined>(undefined);
  latest.current = draft;
  useEffect(() => { if (note && !dirty) { setDraft(note.text); baseRevision.current = store.snapshot?.revision; } }, [note?.text, note, dirty, store.snapshot?.revision]);
  const save = useCallback(async (value = latest.current) => {
    if (composingRef.current) { timer.current = window.setTimeout(() => void save(value), 100); return false; }
    if (!note || value === note.text) { setDirty(false); return true; }
    window.clearTimeout(timer.current); window.clearTimeout(started.current);
    const sentDraft = value;
    const sentBase = baseRevision.current ?? store.snapshot?.revision;
    try {
      const result = await store.updateNote(note, sentDraft, undefined, undefined, sentBase);
      const acknowledged = result.snapshot?.revision ?? store.snapshot?.revision ?? sentBase;
      baseRevision.current = acknowledged;
      if (latest.current === sentDraft) setDirty(false);
      else setDirty(true);
      setLocalError(null); setConflict(false); return true;
    } catch (reason) {
      setDirty(true); const failure = reason as { code?: string; message?: string };
      setConflict(failure.code === 'CONFLICT');
      setLocalError(failure.message ?? (reason instanceof Error ? reason.message : String(reason))); return false;
    }
  }, [note, store.updateNote, store.snapshot?.revision]);
  const change = (value: string) => {
    if (!dirty) baseRevision.current = store.snapshot?.revision;
    setDraft(value); setDirty(true); setLocalError(null); latest.current = value;
    window.clearTimeout(timer.current); timer.current = window.setTimeout(() => void save(value), 300);
    if (started.current === undefined) started.current = window.setTimeout(() => void save(latest.current), 1000);
  };
  useEffect(() => () => { window.clearTimeout(timer.current); window.clearTimeout(started.current); if (dirty && note && latest.current !== note.text) void store.updateNote(note, latest.current).catch(() => undefined); }, [dirty, note?.id]);
  useEffect(() => store.bridge.onFlush(async () => {
    while (composingRef.current) await new Promise((resolve) => window.setTimeout(resolve, 25));
    if (dirty && !(await save())) throw new Error(localError ?? '저장에 실패했습니다.');
  }), [store.bridge, dirty, save, localError]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || !(event.ctrlKey || event.metaKey)) return;
      if (event.shiftKey && event.key.toLowerCase() === 'm') { event.preventDefault(); setRaw((mode) => !mode); }
      if (!event.shiftKey && event.key.toLowerCase() === 'w') { event.preventDefault(); void save().then((saved) => { if (saved) onClose(); }); }
    };
    window.addEventListener('keydown', handler); return () => window.removeEventListener('keydown', handler);
  }, [save, onClose]);
  const updateComposing = (value: boolean) => { composingRef.current = value; if (!value && dirty) void save(); };
  const changeColor = (color: string) => note && void store.updateNote(note, undefined, undefined, color).catch((reason) => setLocalError(String(reason)));
  const addFile = async (file: File) => {
    if (file.size > 20 * 1024 * 1024) { setLocalError('파일은 20 MiB 이하만 첨부할 수 있어요.'); return; }
    const dataUrl = await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result)); reader.onerror = () => reject(new Error('파일을 읽지 못했습니다.')); reader.readAsDataURL(file); });
    try {
      const result = await store.request('importAsset', { name: file.name, mime: file.type || 'application/octet-stream', data: dataUrl.split(',')[1] });
      if (!result.asset) return;
      const view = editorView.current;
      const line = view?.state.doc.lineAt(view.state.selection.main.head);
      const body = `![${file.name.replace(/[\]|\n]/g, '')}](asset:${result.asset.id})`;
      if (view && line) {
        const insert = `${line.text.trim() ? '\n' : ''}${body}\n\n`;
        view.dispatch({ changes: { from: line.to, to: line.to, insert }, selection: { anchor: line.to + insert.length } }); view.focus();
      } else change(`${latest.current}${latest.current ? '\n\n' : ''}${body}\n\n`);
      setLocalError(null);
    } catch (reason) { setLocalError(reason instanceof Error ? reason.message : String(reason)); }
  };
  const overwriteDraft = async () => { baseRevision.current = store.snapshot?.revision; setConflict(false); await save(latest.current); };
  const useRemoteVersion = () => { if (!note) return; setDraft(note.text); latest.current = note.text; baseRevision.current = store.snapshot?.revision; setDirty(false); setConflict(false); setLocalError(null); };
  const deleteNote = async () => {
    if (!note) return;
    try { await store.removeNote(note.id); setDeleteConfirm(false); onClose(); }
    catch (reason) { setLocalError(reason instanceof Error ? reason.message : String(reason)); setDeleteConfirm(false); }
  };
  useEffect(() => {
    if (!deleteConfirm) return;
    cancelDelete.current?.focus();
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); setDeleteConfirm(false); } };
    window.addEventListener('keydown', escape); return () => window.removeEventListener('keydown', escape);
  }, [deleteConfirm]);
  useEffect(() => {
    let alive = true;
    store.assets.forEach((asset) => {
      void store.request('readAsset', { id: asset.id }).then((result) => {
        if (!alive || !result.data) return;
        const binary = atob(result.data); const bytes = new Uint8Array(binary.length); for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
        const url = URL.createObjectURL(new Blob([bytes], { type: result.mime ?? asset.mime })); objectUrls.current.push(url); setAssetUrls((current) => ({ ...current, [asset.id]: url }));
      }).catch(() => undefined);
    });
    return () => { alive = false; };
  }, [store.assets]);
  useEffect(() => () => objectUrls.current.forEach((url) => URL.revokeObjectURL(url)), []);
  if (!note) return <div className="editor-screen"><div className="editor-topbar"><button className="icon-button" onClick={onClose}><Icon name="close"/></button><span>메모를 찾을 수 없습니다</span></div></div>;
  return <div className={`editor-screen note-${note.color}`}>
    <header className="editor-topbar">
      <div className="window-controls"><button aria-label="창 닫기" title="닫기" onClick={onClose}><Icon name="close"/></button><button aria-label="최소화" title="최소화" onClick={() => store.bridge.windowAction('minimize')}><Icon name="minimize"/></button><button aria-label="최대화" title="최대화" onClick={() => store.bridge.windowAction('maximize')}><Icon name="maximize"/></button></div>
      <span className="editor-title">{note.canvas.project ? '프로젝트' : '메모'}</span>
      <div className="editor-tools">
        <button className="icon-button" title="캔버스에서 메모 보기" aria-label="캔버스에서 메모 보기" onClick={() => void store.bridge.openWindow('canvas', note.id)}><Icon name="canvas"/></button>
        <div className="palette-wrap"><button className="icon-button palette-trigger" aria-label="메모 색상" title="메모 색상" onClick={() => setPaletteOpen(!paletteOpen)}><Icon name="palette"/></button>{paletteOpen && <div className="palette-popover" role="menu" aria-label="메모 색상">{Object.entries(NOTE_PALETTE).map(([color, info]) => <button key={color} role="menuitemradio" aria-checked={note.color === color} className={note.color === color ? 'selected' : ''} title={info.label} aria-label={info.label} onClick={() => { changeColor(color); setPaletteOpen(false); }}><span style={{ backgroundColor: info.accent }}>{note.color === color ? '✓' : ''}</span></button>)}</div>}</div>
        <button className="icon-button" title="더보기" aria-label="더보기" onClick={() => setMenu(!menu)}><Icon name="more"/></button>
        <input ref={fileInput} hidden type="file" onChange={(event) => { const file = event.target.files?.[0]; if (file) void addFile(file); event.target.value = ''; }}/>
      </div>
      {menu && <div className="editor-menu"><button onClick={() => { setRaw(!raw); setMenu(false); }}>원문 {raw ? '숨기기' : '보기'} <kbd>Ctrl ⇧ M</kbd></button><button onClick={() => { void store.bridge.openWindow('files'); setMenu(false); }}>파일</button><button onClick={() => { setMenu(false); setDeleteConfirm(true); }}>삭제</button><button onClick={() => { setMenu(false); window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1' })); }}>단축키·서식 도움말 <kbd>F1</kbd></button></div>}
    </header>
    <div className="editor-content" onPaste={(event) => { const text = event.clipboardData.getData('text/plain'); if (text) return; const file = Array.from(event.clipboardData.items).map((item) => item.getAsFile()).find((item) => item?.type.startsWith('image/')); if (file) { event.preventDefault(); void addFile(file); } }} onDragOver={(event) => { if (event.dataTransfer.files.length) event.preventDefault(); }} onDrop={(event) => { const file = Array.from(event.dataTransfer.files).find((item) => item.type.startsWith('image/')); if (file) { event.preventDefault(); void addFile(file); } }}><MarkdownSurface value={draft} onChange={change} raw={raw} assetUrls={assetUrls} onReady={(view) => { editorView.current = view; if (view) { view.dispatch({ selection: { anchor: view.state.doc.length }, scrollIntoView: false }); view.focus(); } }} onComposing={updateComposing} onExternal={(url) => { void store.bridge.openExternal(url); }}/></div>
    <footer className={`editor-status${localError ? ' save-error' : ''}`}>{localError ? <><span>{localError}</span>{conflict ? <><button onClick={() => void overwriteDraft()}>내 초안으로 덮어쓰기</button><button onClick={useRemoteVersion}>최신 버전 불러오기</button></> : <button onClick={() => void save()}>다시 시도</button>}</> : <span>{store.saving ? '저장 중' : dirty ? '변경 사항' : '저장됨'}</span>}</footer>
    {deleteConfirm && <div className="modal-backdrop" role="presentation"><section className="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="note-delete-title"><h2 id="note-delete-title">메모를 삭제할까요?</h2><p>삭제하면 메모를 복구할 수 없습니다.</p><div><button ref={cancelDelete} className="secondary-button" onClick={() => setDeleteConfirm(false)}>취소</button><button className="danger-button" onClick={() => void deleteNote()}>삭제</button></div></section></div>}
  </div>;
}
