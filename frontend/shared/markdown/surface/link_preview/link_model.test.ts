import { describe, expect, it } from 'vitest';
import { externalLinkUrl, findLinkAt, linkInfo } from './link_model';

describe('findLinkAt', () => {
  it('returns the destination range and label for a Markdown link', () => {
    const text = 'open [Memo](https://example.com/a?q=1) now';
    const from = text.indexOf('https');
    const url = 'https://example.com/a?q=1';
    const link = { from, to: from + url.length, url, label: 'Memo' };
    expect(findLinkAt(text, from + 2)).toEqual(link);
    expect(findLinkAt(text, text.indexOf('Memo'))).toEqual(link);
  });

  it.each([
    '![image.png|161](asset:abcdef)',
    '![](https://example.com/image.png)',
    '![이미지](https://example.com/image.png)',
  ])('finds no link inside image markup %s', (image) => {
    const text = `${image} [네이버](naver.com)`;
    for (let position = 0; position < image.length; position += 1) {
      expect(findLinkAt(text, position)).toBeNull();
    }
    expect(findLinkAt(text, text.indexOf('네이버'))?.url).toBe('naver.com');
  });

  it('preserves links after an escaped exclamation mark', () => {
    const text = String.raw`\![메모](https://example.com)`;
    expect(findLinkAt(text, text.indexOf('메모'))?.url).toBe('https://example.com');
  });

  it('finds bare URLs and excludes trailing punctuation', () => {
    const text = 'See https://example.com/path.';
    expect(findLinkAt(text, text.indexOf('path'))).toEqual({ from: 4, to: 28, url: 'https://example.com/path', label: '' });
  });

  it('preserves the Naver destination and uses HTTPS when opening it', () => {
    const url = 'naver.com';
    const text = '[링크](https://example.com)\n[네이버](naver.com)';
    const from = text.indexOf(url);
    expect(findLinkAt(text, text.indexOf('네이버'))).toEqual({ from, to: from + url.length, url, label: '네이버' });
    expect(linkInfo(url, '네이버')).toMatchObject({ title: '네이버', address: url, warning: '' });
    expect(externalLinkUrl(url)).toBe('https://naver.com');
    expect(externalLinkUrl('mailto:team@example.com')).toBe('mailto:team@example.com');
  });

  it('returns null outside link text and formats mail addresses', () => {
    expect(findLinkAt('plain text', 4)).toBeNull();
    expect(linkInfo('mailto:team@example.com', '')).toMatchObject({ mail: true, badge: '@', warning: '' });
    expect(linkInfo('http://example.com/path', '')).toMatchObject({ title: 'example.com', address: '/path' });
    expect(linkInfo('http://example.com/path', '').warning).toContain('보안 연결');
  });
});
