import type { ReactNode } from 'react';
import type { TokenGroup } from './readTokens';

type Render = (name: string) => ReactNode;

/** 라이트·다크 견본. 색은 light-dark()로 정해지므로 테마 클래스만 바꿔 같은 토큰을 두 번 칠한다. */
const colorPair: Render = (name) => (
  <span className="dsb-swatch" aria-hidden="true">
    <span className="theme-light" style={{ background: `var(${name})` }}/>
    <span className="theme-dark" style={{ background: `var(${name})` }}/>
  </span>
);

/** 토큰 묶음별 미리보기 전략. 묶음을 더하면 한 줄을 더한다. 없는 묶음은 이름·값만 보인다. */
export const SWATCHES: Partial<Record<TokenGroup['id'], Render>> = {
  surface: colorPair,
  note: colorPair,
  dot: colorPair,
  type: (name) => <span style={{ fontSize: `var(${name})` }}>가Aa</span>,
};
