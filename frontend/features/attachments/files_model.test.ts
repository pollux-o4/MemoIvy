import { describe, expect, it } from 'vitest';
import { daysAgo, readableBytes, reference, usersByAsset } from './files_model';

describe('files view display values', () => {
  it('copies the GTK image reference syntax', () => {
    expect(reference({ id: 'a'.repeat(64) })).toBe(`![](asset:${'a'.repeat(64)})`);
  });

  it('formats bytes like the GTK detail panel', () => {
    expect(readableBytes(0)).toBe('0 B');
    expect(readableBytes(1536)).toBe('2 KB');
    expect(readableBytes(1_572_864)).toBe('1.5 MB');
  });

  it('uses elapsed full days for last-use labels', () => {
    const now = 1_800_000_000;
    expect(daysAgo(undefined, now)).toBe('오늘');
    expect(daysAgo(now - 86_399, now)).toBe('오늘');
    expect(daysAgo(now - 86_400, now)).toBe('어제');
    expect(daysAgo(now - 3 * 86_400, now)).toBe('3일 전');
  });

  it('uses exact asset references and orders users by most recently edited', () => {
    const id = 'b'.repeat(64);
    const users = usersByAsset([
      { id: 'old', text: `![x](asset:${id})`, modified: 10 },
      { id: 'new', text: `broken asset:${id} and asset:${id}`, modified: 20 },
      { id: 'wrong-case', text: `asset:${id.toUpperCase()}`, modified: 30 },
    ]);
    expect(users.get(id)).toEqual(['new', 'old']);
  });
});
