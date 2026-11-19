import type { ReactNode } from 'react';

/** 접힘 펼침 공통 모션(motion.css `.reveal`). 닫힌 내용은 DOM에 남되 inert라 초점·클릭이 닿지 않는다. */
export function Reveal({ open, className = '', children }: { open: boolean; className?: string; children: ReactNode }) {
  return <div className={`reveal${open ? ' is-open' : ''} ${className}`.trim()} inert={!open}><div>{children}</div></div>;
}
