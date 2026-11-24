import { useEffect, useState } from 'react';

/** GTK .memoivy-window 상태: 비활성(backdrop)이면 제목을 흐리게, 최대화면 모서리·그림자를 없앤다. */
/** desktop/window_chrome.cjs와 같은 규칙: 투명(둥근) 창은 Linux만. */
const roundedFrame = typeof navigator === 'undefined' || !/^(Win|Mac)/.test(navigator.platform);

export function useWindowFrame(): string {
  const [focused, setFocused] = useState(() => typeof document === 'undefined' || document.hasFocus());
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    const focus = () => setFocused(true);
    const blur = () => setFocused(false);
    // 프레임 없는 창은 최대화 이벤트가 화면에 오지 않아, 창이 작업 영역을 가득 채웠는지로 판단한다.
    const resize = () => setMaximized(window.outerWidth >= screen.availWidth && window.outerHeight >= screen.availHeight);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);
    window.addEventListener('resize', resize);
    resize();
    return () => { window.removeEventListener('focus', focus); window.removeEventListener('blur', blur); window.removeEventListener('resize', resize); };
  }, []);
  return `${focused ? '' : ' is-backdrop'}${maximized ? ' is-maximized' : ''}${roundedFrame ? '' : ' is-square-frame'}`;
}
