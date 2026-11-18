import { describe, expect, it } from 'vitest';
import { deleteImageLine, displayImageSize, imageMarkup, moveImageLine, resizeImageWidth } from './commands';

describe('inline image commands', () => {
  it('keeps the original aspect ratio and clamps resize to available text width', () => {
    expect(displayImageSize(undefined, { width: 800, height: 400 }, 300)).toEqual({ width: 300, height: 150 });
    expect(resizeImageWidth(100, 100, 0, 0.5, 240)).toBe(150);
    expect(resizeImageWidth(100, -200, 0, 0.5, 240)).toBe(64);
  });

  it('writes the existing asset reference syntax', () => {
    expect(imageMarkup('설명', 120, 'a'.repeat(64))).toBe(`![설명|120](asset:${'a'.repeat(64)})`);
  });

  it('removes an image line without leaving an extra newline', () => {
    expect(deleteImageLine('앞\n이미지\n뒤', 2, 5)).toBe('앞\n뒤');
    expect(deleteImageLine('이미지\n', 0, 3)).toBe('');
  });

  it('moves an image reference before the target line', () => {
    expect(moveImageLine('첫째\n이미지\n셋째\n', 3, 10)).toBe('첫째\n셋째\n이미지\n');
    expect(moveImageLine('첫째\n이미지\n셋째', 3, 0)).toBe('이미지\n첫째\n셋째');
  });
});
