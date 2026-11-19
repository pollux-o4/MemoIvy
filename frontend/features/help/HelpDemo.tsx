/**
 * 기본 동작을 포인터 움직임으로 보여 주는 작은 SVG. 움직임은 help.css 키프레임만 맡고, 모션 줄이기면 멈춘 그림으로 남는다.
 * drag: 카드 끌어 옮기기 · connect: 손잡이 끌어 잇기 · pan: Space+끌기로 보드 이동 · open: 두 번 눌러 열기.
 */
export type DemoKind = 'drag' | 'connect' | 'pan' | 'open';

const POINTER = 'M0 0 L0 13 L3.5 10 L6 15.5 L8 14.6 L5.6 9.3 L10 9.3 Z';

export function HelpDemo({ kind }: { kind: DemoKind }) {
  return <svg className={`help-demo demo-${kind}`} viewBox="0 0 120 64" aria-hidden="true">
    {kind === 'pan' && <g className="demo-board">{[20, 44, 68, 92].map((x) => [16, 34, 52].map((y) => <circle key={`${x}-${y}`} cx={x} cy={y} r="1.2"/>))}<rect className="demo-card" x="40" y="20" width="30" height="20" rx="3"/></g>}
    {kind === 'connect' && <><rect className="demo-card" x="8" y="22" width="30" height="20" rx="3"/><rect className="demo-card is-target" x="82" y="22" width="30" height="20" rx="3"/><circle className="demo-handle" cx="38" cy="32" r="3"/><path className="demo-line" d="M38 32 H82" pathLength="1"/></>}
    {kind === 'drag' && <rect className="demo-card demo-moving" x="14" y="22" width="30" height="20" rx="3"/>}
    {kind === 'open' && <><rect className="demo-card" x="45" y="22" width="30" height="20" rx="3"/><circle className="demo-ripple" cx="60" cy="32" r="4"/></>}
    <path className="demo-pointer" d={POINTER}/>
  </svg>;
}
