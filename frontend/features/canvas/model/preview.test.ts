import { describe, expect, test } from 'vitest';
import { canvasPreview } from './preview';

describe('canvas markdown preview', () => {
  test('retains title markers, check state, numbering, and overflow count', () => {
    const result = canvasPreview('- [x] First\n- [ ] Second\n1. Third\n- Fourth\nFifth', 3);
    expect(result.title).toMatchObject({ kind: 'task', done: true, text: 'First' });
    expect(result.rows.map(({ kind }) => kind)).toEqual(['task', 'number', 'bullet']);
    expect(result.remaining).toBe(1);
  });

  test('keeps each line break as its own row', () => {
    expect(canvasPreview('Title\nFirst\nSecond', 3).rows).toEqual([{ kind: 'text', text: 'First' }, { kind: 'text', text: 'Second' }]);
  });
});
