import { describe, expect, test } from 'vitest';
import { markdownPreview, togglePreviewTask } from './model';

describe('markdown preview', () => {
  test('retains title markers, check state, numbering, and overflow count', () => {
    const result = markdownPreview('- [x] First\n- [ ] Second\n1. Third\n- Fourth\nFifth', 3);
    expect(result.title).toMatchObject({ kind: 'task', task: 'x', text: 'First' });
    expect(result.rows.map(({ kind }) => kind)).toEqual(['task', 'number', 'bullet']);
    expect(result.remaining).toBe(1);
  });

  test('keeps each line break as its own row', () => {
    expect(markdownPreview('Title\nFirst\nSecond', 3).rows).toEqual([{ kind: 'text', text: 'First', depth: 0 }, { kind: 'text', text: 'Second', depth: 0 }]);
  });

  test('toggles the exact duplicate task after blank lines without changing its Markdown', () => {
    const markdown = 'Title\n- [ ] same\n\n  +   [/]   same [label]\r\n- [X] last';
    const row = markdownPreview(markdown).rows[1];
    const line = row.kind === 'task' ? row.sourceLine : undefined;
    expect(line).toBe(3);
    const checked = togglePreviewTask(markdown, line ?? -1);
    expect(checked).toBe(markdown.replace('[/]', '[x]'));
    expect(togglePreviewTask(checked, 4)).toBe(checked.replace('[X]', '[ ]'));
    expect(togglePreviewTask(checked, 2)).toBe(checked);
  });

  test('keeps literal checkbox examples inside fences read-only', () => {
    const markdown = 'Title\n```markdown\n- [ ] example\n```\n- [ ] actual';
    expect(markdownPreview(markdown).rows.filter((row) => row.kind === 'task').map(({ sourceLine }) => sourceLine)).toEqual([4]);
    expect(togglePreviewTask(markdown, 2)).toBe(markdown);
  });

  test('keeps the editor list depth for nested items', () => {
    expect(markdownPreview('T\n- [ ] 위\n  - [ ] 아래', 3).rows.map((row) => row.depth)).toEqual([0, 1]);
  });
});
