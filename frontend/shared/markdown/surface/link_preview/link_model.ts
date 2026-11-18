/** 링크 미리보기의 순수 규칙: 커서 위치의 링크 찾기와 표시 정보. */
export type LinkTarget = { from: number; to: number; url: string; label: string };

/** Finds inline Markdown links and bare web/mail URLs without depending on parser node names. */
export function findLinkAt(text: string, position: number): LinkTarget | null {
  const links: LinkTarget[] = [];
  const markdown = /\[([^\]]+)\]\(\s*(<[^>]+>|[^\s)]+)(?:\s+[^)]*)?\s*\)/g;
  for (const match of text.matchAll(markdown)) {
    const raw = match[2];
    const url = raw.startsWith('<') && raw.endsWith('>') ? raw.slice(1, -1) : raw;
    const start = (match.index ?? 0) + match[0].indexOf(raw) + (raw.startsWith('<') ? 1 : 0);
    links.push({ from: start, to: start + url.length, url, label: match[1] });
  }
  const bare = /(?:https?:\/\/|mailto:)[^\s<>()[\]]+/gi;
  for (const match of text.matchAll(bare)) {
    const from = match.index ?? 0;
    const to = from + match[0].replace(/[.,!?;:]$/, '').length;
    if (!links.some((link) => from < link.to && to > link.from)) {
      links.push({ from, to, url: text.slice(from, to), label: '' });
    }
  }
  return links.find(({ from, to }) => position >= from && position <= to) ?? null;
}

export function linkInfo(url: string, label: string) {
  let host = url;
  let rest = '';
  const mail = url.toLowerCase().startsWith('mailto:');
  try {
    const parsed = new URL(url);
    host = mail ? parsed.pathname.split('@').at(-1) || url : parsed.host;
    rest = mail ? '' : `${parsed.pathname}${parsed.search}${parsed.hash}`;
  } catch {
    // Keep unsupported or malformed destinations visible so the user can inspect them.
  }
  const named = label.trim() !== '' && label !== url;
  const warning = !mail && !url.toLowerCase().startsWith('https://')
    ? '보안 연결이 아닌 주소입니다. 열기 전에 주소를 확인하세요.'
    : '';
  const initial = host.replace(/^www\./i, '').charAt(0).toLocaleUpperCase() || '?';
  return { host, rest, mail, named, title: named ? label.trim() : host, address: named ? `${host}${rest}` : rest, badge: mail ? '@' : initial, warning };
}
