import { useMemo } from 'react';
import { readTokens } from './readTokens';
import { SWATCHES } from './swatches';

/** shared/tokens.css의 선언값을 그대로 보여 준다. */
export function TokenPanel() {
  const groups = useMemo(() => readTokens(), []);
  return (
    <div className="dsb-grid">
      {groups.map((group) => (
        <section key={group.id} aria-label={`${group.label} 토큰`} className="dsb-card">
          <h2>{group.label}</h2>
          <ul>
            {group.tokens.map((token) => (
              <li key={token.name} className="dsb-token">
                {SWATCHES[group.id]?.(token.name) ?? <span/>}
                <code>{token.name}</code>
                <span className="dsb-value" title={token.value}>{token.value}</span>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
