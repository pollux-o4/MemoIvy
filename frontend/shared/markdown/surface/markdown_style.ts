import { HighlightStyle } from '@codemirror/language';
import { tags } from '@lezer/highlight';

/** 마크다운 제목: '#'·'##'만 제목(1.6배·1.3배, 700), 기호는 흐리게. '###' 이하는 본문 그대로. */
export const markdownStyle = HighlightStyle.define([
  { tag: tags.heading1, fontSize: '1.6em', fontWeight: '700', lineHeight: '1.3' },
  { tag: tags.heading2, fontSize: '1.3em', fontWeight: '700', lineHeight: '1.35' },
  { tag: tags.strong, fontWeight: '700' },
  { tag: tags.emphasis, fontStyle: 'italic' },
  { tag: tags.strikethrough, textDecoration: 'line-through' },
  { tag: tags.link, textDecoration: 'underline', textUnderlineOffset: '3px' },
  { tag: tags.url, color: 'var(--note-muted)' },
  { tag: tags.processingInstruction, color: 'var(--note-muted)', fontWeight: '400' },
  { tag: tags.monospace, fontFamily: 'ui-monospace, "Noto Sans Mono CJK KR", monospace', fontSize: '.92em' },
]);
