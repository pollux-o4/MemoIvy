import { describe, expect, it } from 'vitest';
import { findLinkAt, linkInfo } from './link_model';

describe('findLinkAt', () => {
  it('returns the destination range and label for a Markdown link', () => {
    const text = 'open [Memo](https://example.com/a?q=1) now';
    const from = text.indexOf('https');
    const url = 'https://example.com/a?q=1';
    const link = { from, to: from + url.length, url, label: 'Memo' };
    expect(findLinkAt(text, from + 2)).toEqual(link);
    expect(findLinkAt(text, text.indexOf('Memo'))).toEqual(link);
  });

  it('finds bare URLs and excludes trailing punctuation', () => {
    const text = 'See https://example.com/path.';
    expect(findLinkAt(text, text.indexOf('path'))).toEqual({ from: 4, to: 28, url: 'https://example.com/path', label: '' });
  });

  it('finds the Naver URL below a named link', () => {
    const url = 'https://naver.com';
    const text = `[링크](https://example.com)\n${url}`;
    const from = text.indexOf(url);
    expect(findLinkAt(text, from + 8)).toEqual({ from, to: text.length, url, label: '' });
  });

  it('returns null outside link text and formats mail addresses', () => {
    expect(findLinkAt('plain text', 4)).toBeNull();
    expect(linkInfo('mailto:team@example.com', '')).toMatchObject({ mail: true, badge: '@', warning: '' });
    expect(linkInfo('http://example.com/path', '')).toMatchObject({ title: 'example.com', address: '/path' });
    expect(linkInfo('http://example.com/path', '').warning).toContain('보안 연결');
  });
});
