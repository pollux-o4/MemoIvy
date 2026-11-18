import { EditorView } from '@codemirror/view';

export const editorTheme = EditorView.theme({
  '&': { height: '100%', fontSize: '13.333px', color: 'var(--note-text)' },
  '.cm-scroller': { fontFamily: 'inherit', lineHeight: '1.5', overflow: 'auto', padding: '20px 0 24px' },
  // 선택·커서는 시스템 기본(GTK처럼 글자 범위만 칠함). 깜빡임은 motion.css의 caret-fade가 맡는다.
  '.cm-content': { maxWidth: 'none', margin: '0', padding: '0 20px', '--caret': 'var(--memoivy-accent)', caretColor: 'var(--memoivy-accent)' },
  '.cm-line': { padding: '0' },
  '::selection': { background: 'color-mix(in srgb, var(--memoivy-accent) 28%, transparent)' },
  '.cm-activeLine': { backgroundColor: 'transparent' },
  '.cm-gutters': { display: 'none' },
  '.cm-focused': { outline: 'none' },
  '.cm-mark-hidden': { display: 'none' },
});
