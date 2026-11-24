import type { ReactNode } from 'react';
import type { DesktopBridge } from './engine_contract';
import { Icon } from './Icon';

type WindowControl = 'minimize' | 'maximize';

/** 모든 창의 제목 줄: 왼쪽 창 조작 · 가운데 제목 · 오른쪽 동작. 프레임 없는 창의 끌기 영역도 여기서 정한다. */
export function WindowHeader({ title, bridge, onClose, controls = ['minimize', 'maximize'], actions, className = 'window-header' }: {
  title: ReactNode; bridge: DesktopBridge; onClose: () => void; controls?: WindowControl[]; actions?: ReactNode; className?: string;
}) {
  // biome-ignore lint/a11y/noStaticElementInteractions: 제목 줄 더블클릭 최대화는 포인터 편의다. 키보드는 최대화 버튼을 쓴다.
  return <header className={className} onDoubleClick={(event) => {
    if (!(event.target as HTMLElement).closest('button, a, input, textarea, [contenteditable="true"], [role="button"]')) bridge.windowAction('maximize');
  }}>
    <div className="window-controls">
      <button aria-label="창 닫기" title="닫기" onClick={onClose}><Icon name="close"/></button>
      {controls.includes('minimize') && <button aria-label="최소화" title="최소화" onClick={() => bridge.windowAction('minimize')}><Icon name="minimize"/></button>}
      {controls.includes('maximize') && <button aria-label="최대화" title="최대화" onClick={() => bridge.windowAction('maximize')}><Icon name="maximize"/></button>}
    </div>
    <span className="window-title">{title}</span>
    <div className="header-actions">{actions}</div>
  </header>;
}
