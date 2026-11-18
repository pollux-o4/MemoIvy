import { useEffect, useMemo, useRef } from 'react';
import { indentWithTab, history, historyKeymap, defaultKeymap } from '@codemirror/commands';
import { Compartment, EditorState } from '@codemirror/state';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { syntaxHighlighting } from '@codemirror/language';
import { markdownStyle } from './markdown_style';
import { EditorView, keymap } from '@codemirror/view';
import { editorTheme } from './editor_theme';
import { formattingKeymap } from './formatting_keymap';
import { liveMarkdown } from './live_markdown';
import { linkPreview } from './link_preview/link_preview';
import type { ImageHandlers } from './image_tools/widget';

export function MarkdownSurface({ value, onChange, raw, assetUrls, onReady, onComposing, onExternal, onViewImage }: {
  value: string;
  onChange: (value: string) => void;
  raw: boolean;
  assetUrls: Record<string, string>;
  onReady: (view: EditorView | null) => void;
  onComposing: (value: boolean) => void;
  onExternal: (url: string) => void;
  onViewImage: (hash: string) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const rawMode = useMemo(() => new Compartment(), []);
  const callbacks = useRef({ onChange, onReady, onComposing, onExternal, onViewImage });
  callbacks.current = { onChange, onReady, onComposing, onExternal, onViewImage };
  // GTK 빼기는 문서에서 참조 줄만 지우므로 앱 쪽 추가 동작이 없다.
  const imageHandlers = useMemo<ImageHandlers>(() => ({ view: (hash) => callbacks.current.onViewImage(hash), extract: () => {} }), []);

  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({ doc: value, extensions: [
      history(), EditorView.lineWrapping, EditorView.contentAttributes.of({ 'aria-label': '메모 내용' }), markdown({ base: markdownLanguage }), syntaxHighlighting(markdownStyle), editorTheme, linkPreview({ open: (url) => callbacks.current.onExternal(url) }),
      rawMode.of(raw ? [] : [liveMarkdown(assetUrls, imageHandlers)]),
      keymap.of([...formattingKeymap(), ...historyKeymap, ...defaultKeymap, indentWithTab]),
      EditorView.domEventHandlers({
        compositionstart: () => { callbacks.current.onComposing(true); return false; },
        compositionend: () => { window.setTimeout(() => callbacks.current.onComposing(false), 0); return false; },
        click: (event, currentView) => {
          if (!(event.ctrlKey || event.metaKey)) return false;
          const position = currentView.posAtCoords({ x: event.clientX, y: event.clientY });
          if (position === null) return false;
          const line = currentView.state.doc.lineAt(position);
          const match = /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/.exec(line.text);
          if (!match) return false;
          callbacks.current.onExternal(match[2]);
          return true;
        },
      }),
      EditorView.updateListener.of((update) => { if (update.docChanged) callbacks.current.onChange(update.state.doc.toString()); }),
    ] });
    const editor = new EditorView({ state, parent: host.current });
    view.current = editor;
    callbacks.current.onReady(editor);
    return () => { callbacks.current.onReady(null); editor.destroy(); view.current = null; };
  }, []);

  useEffect(() => { view.current?.dispatch({ effects: rawMode.reconfigure(raw ? [] : [liveMarkdown(assetUrls, imageHandlers)]) }); }, [raw, assetUrls, rawMode, imageHandlers]);
  useEffect(() => {
    const editor = view.current;
    if (editor && value !== editor.state.doc.toString()) editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
  }, [value]);

  return <div className={`markdown-surface${raw ? ' raw-mode' : ''}`} ref={host} />;
}
