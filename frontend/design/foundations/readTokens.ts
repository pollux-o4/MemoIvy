/** 토큰 묶음. 이름 접두사로 나누며 값은 shared/tokens.css가 유일한 출처다. */
export interface TokenGroup {
  id: 'surface' | 'note' | 'dot' | 'type';
  label: string;
  tokens: { name: string; value: string }[];
}

const GROUPS: { id: TokenGroup['id']; label: string; match: RegExp }[] = [
  { id: 'surface', label: '앱 표면', match: /^--memoivy-/ },
  { id: 'note', label: '메모 색', match: /^--note-/ },
  { id: 'dot', label: '색 점', match: /^--dot-/ },
  { id: 'type', label: '글자', match: /^--font-/ },
];

/** 문서에 로드된 :root 규칙에서 토큰을 읽는다. 토큰을 여기 다시 적지 않는다. */
export function readTokens(sheets: Iterable<CSSStyleSheet> = document.styleSheets): TokenGroup[] {
  const declared = new Map<string, string>();
  for (const sheet of sheets) {
    let rules: CSSRuleList;
    try { rules = sheet.cssRules; } catch { continue; } // 다른 출처 스타일시트는 읽을 수 없다.
    for (const rule of rules) {
      if (!(rule instanceof CSSStyleRule) || rule.selectorText !== ':root') continue;
      for (const name of rule.style) if (name.startsWith('--')) declared.set(name, rule.style.getPropertyValue(name).trim());
    }
  }
  return GROUPS.map(({ id, label, match }) => ({
    id,
    label,
    tokens: [...declared].filter(([name]) => match.test(name)).map(([name, value]) => ({ name, value })),
  }));
}
