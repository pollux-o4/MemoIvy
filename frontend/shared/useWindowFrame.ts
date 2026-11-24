import { useEffect, useState } from 'react';
import type { DesktopBridge } from './engine_contract';

/** desktop/window_chrome.cjs와 같은 규칙: 투명(둥근) 창은 Linux만. */
const roundedFrame = typeof navigator === 'undefined' || !/^(Win|Mac)/.test(navigator.platform);

/** GTK .memoivy-window 상태: 비활성(backdrop)이면 제목을 흐리게, 최대화면 모서리·그림자를 없앤다. 최대화 여부는 메인 프로세스가 알린다. */
export function useWindowFrame(bridge: Pick<DesktopBridge, 'onWindowState'>): string {
  const [focused, setFocused] = useState(() => typeof document === 'undefined' || document.hasFocus());
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    const focus = () => setFocused(true);
    const blur = () => setFocused(false);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);
    return () => { window.removeEventListener('focus', focus); window.removeEventListener('blur', blur); };
  }, []);
  useEffect(() => bridge.onWindowState((state) => setMaximized(state.maximized)), [bridge]);
  return `${focused ? '' : ' is-backdrop'}${maximized ? ' is-maximized' : ''}${roundedFrame ? '' : ' is-square-frame'}`;
}
