import { useEffect, useState } from 'react';
import type { DesktopBridge, WindowState } from './engine_contract';

/** GTK .memoivy-window 상태: 비활성(backdrop)이면 제목을 흐리게, 최대화면 모서리·그림자를 없앤다. 창 상태는 메인 프로세스가 알린다. */
export function useWindowFrame(bridge: Pick<DesktopBridge, 'onWindowState'>): string {
  const [focused, setFocused] = useState(() => typeof document === 'undefined' || document.hasFocus());
  const [state, setState] = useState<WindowState>({ maximized: false, rounded: true });
  useEffect(() => {
    const focus = () => setFocused(true);
    const blur = () => setFocused(false);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);
    return () => { window.removeEventListener('focus', focus); window.removeEventListener('blur', blur); };
  }, []);
  useEffect(() => bridge.onWindowState(setState), [bridge]);
  return `${focused ? '' : ' is-backdrop'}${state.maximized ? ' is-maximized' : ''}${state.rounded ? '' : ' is-square-frame'}`;
}
