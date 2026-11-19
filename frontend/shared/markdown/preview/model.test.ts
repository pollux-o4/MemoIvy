import { describe, expect, test } from 'vitest';
import { markdownPreview, rebasePreviewTask, togglePreviewTask } from './model';

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

  test.each([
    { name: '백틱과 언어 이름', fence: ['```markdown', '- [ ] 예시', '```'] },
    { name: '짧은 백틱과 다른 종류의 펜스', fence: ['````md', '```', '- [ ] 예시', '~~~~', '- [ ] 다른 예시', '````'] },
    { name: '닫는 기호 뒤에 글자가 있는 경우', fence: ['~~~md', '~~~ 아직 코드', '- [ ] 예시', '~~~  '] },
    { name: '닫히지 않은 코드 블록', fence: ['```', '- [ ] 예시'], unclosed: true },
  ])('$name 안의 체크박스 예시는 원문을 바꾸지 않는다', ({ fence, unclosed }) => {
    const prefix = ['제목', '> - [ ] 인용', '- [/] **한글**과 `- [ ]`', '  - [x] [링크](https://example.com)'];
    const lines = [...prefix, ...fence, '- [ ] 실제 할 일'];
    const markdown = lines.join('\r\n');
    const expected = unclosed ? [2, 3] : [2, 3, lines.length - 1];
    const tasks = markdownPreview(markdown).rows.filter((row) => row.kind === 'task');

    expect(tasks.map(({ sourceLine }) => sourceLine)).toEqual(expected);
    for (const [index, line] of lines.entries()) {
      const changed = [...lines];
      if (expected.includes(index)) changed[index] = line.replace(/\[([ x/])\]/, (_, state) => state === 'x' ? '[ ]' : '[x]');
      expect(togglePreviewTask(markdown, index)).toBe(changed.join('\r\n'));
    }
  });

  test('rebases consecutive toggles and rejects edits to the original layout', () => {
    const original = 'Title\n- [ ] first\n- [ ] second';
    const first = togglePreviewTask(original, 1);
    expect(rebasePreviewTask(original, first, 2)).toBe('Title\n- [x] first\n- [x] second');
    expect(rebasePreviewTask(original, first, 1)).toBe(original);
    expect(rebasePreviewTask(original, first.replace('second', 'edited'), 2)).toBeUndefined();
    expect(rebasePreviewTask(original, `${first}\n- [ ] new`, 2)).toBeUndefined();
  });

  test('keeps the editor list depth for nested items', () => {
    expect(markdownPreview('T\n- [ ] 위\n  - [ ] 아래', 3).rows.map((row) => row.depth)).toEqual([0, 1]);
  });
});
