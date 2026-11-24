import { useEffect, useMemo, useRef } from 'react';
import './markdown_surface.css';
import { indentWithTab, history, historyKeymap, defaultKeymap } from '@codemirror/commands';
import { Compartment, EditorState, Prec } from '@codemirror/state';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { syntaxHighlighting } from '@codemirror/language';
import { markdownStyle } from './markdown_style';
import { EditorView, keymap, placeholder as hint } from '@codemirror/view';
import { editorTheme } from './editor_theme';
import { formattingKeymap } from './formatting_keymap';
import { pendingFence } from './fence_input';
import { liveMarkdown } from './live_markdown';
import { linkPreview } from './link_preview/link_preview';
import { externalLinkUrl, findLinkAt } from './link_preview/link_model';
import type { ImageHandlers } from './image_tools/widget';

export function MarkdownSurface({ value, onChange, raw, assetUrls, onReady, onComposing, onExternal, onViewImage, label = '메모 내용', placeholder, compact = false }: {
  value: string;
  /** 편집 영역의 접근성 이름. */
  label?: string;
  placeholder?: string;
  /** 좁은 칸에 넣는 작은 여백 변형. */
  compact?: boolean;
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
  const hints = useMemo(() => new Compartment(), []);
  const hintExtensions = () => [EditorView.contentAttributes.of({ 'aria-label': label }), placeholder ? hint(placeholder) : []];
  const callbacks = useRef({ onChange, onReady, onComposing, onExternal, onViewImage });
  callbacks.current = { onChange, onReady, onComposing, onExternal, onViewImage };
  // GTK 빼기는 문서에서 참조 줄만 지우므로 앱 쪽 추가 동작이 없다.
  const imageHandlers = useMemo<ImageHandlers>(() => ({ view: (hash) => callbacks.current.onViewImage(hash), extract: () => {} }), []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: 편집기는 한 번만 만든다. 이후 값·모드·안내 변경은 아래 효과들이 reconfigure로 반영한다.
  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({ doc: value, extensions: [
      history(), EditorView.lineWrapping, hints.of(hintExtensions()), markdown({ base: markdownLanguage }), syntaxHighlighting(markdownStyle), editorTheme, linkPreview({ open: (url) => callbacks.current.onExternal(url) }),
      rawMode.of(raw ? [] : [liveMarkdown(assetUrls, imageHandlers)]),
      pendingFence,
      Prec.highest(keymap.of(formattingKeymap())),
      keymap.of([...historyKeymap, ...defaultKeymap, indentWithTab]),
      EditorView.domEventHandlers({
        compositionstart: () => { callbacks.current.onComposing(true); return false; },
        compositionend: () => { window.setTimeout(() => callbacks.current.onComposing(false), 0); return false; },
        click: (event, currentView) => {
          if (!(event.ctrlKey || event.metaKey)) return false;
          const position = currentView.posAtCoords({ x: event.clientX, y: event.clientY });
          if (position === null) return false;
          const link = findLinkAt(currentView.state.doc.toString(), position);
          if (!link) return false;
          callbacks.current.onExternal(externalLinkUrl(link.url));
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

  // biome-ignore lint/correctness/useExhaustiveDependencies: 이름·안내 문구가 바뀔 때만 다시 건다
  useEffect(() => { view.current?.dispatch({ effects: hints.reconfigure(hintExtensions()) }); }, [label, placeholder, hints]);
  useEffect(() => { view.current?.dispatch({ effects: rawMode.reconfigure(raw ? [] : [liveMarkdown(assetUrls, imageHandlers)]) }); }, [raw, assetUrls, rawMode, imageHandlers]);
  useEffect(() => {
    const editor = view.current;
    if (editor && value !== editor.state.doc.toString()) editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
  }, [value]);

  return <div className={`markdown-surface${raw ? ' raw-mode' : ''}${compact ? ' compact' : ''}`} ref={host} />;
}
