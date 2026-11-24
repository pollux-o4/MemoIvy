import { useEffect, useEffectEvent } from 'react';

/** 마우스 뒤로 버튼(MouseEvent.button 3). Chromium은 Linux·Windows 모두 이 값으로 전달한다. */
const MOUSE_BACK_BUTTON = 3;

/** 마우스 뒤로 버튼을 화면의 '뒤로' 명령에 연결한다. 기본 동작(문서 기록 이동)은 막는다. */
export function useMouseBack(goBack: () => void): void {
  const onBack = useEffectEvent(goBack);
  useEffect(() => {
    const swallow = (event: MouseEvent) => { if (event.button === MOUSE_BACK_BUTTON) event.preventDefault(); };
    const release = (event: MouseEvent) => {
      if (event.button !== MOUSE_BACK_BUTTON) return;
      event.preventDefault();
      onBack();
    };
    window.addEventListener('mousedown', swallow);
    window.addEventListener('mouseup', release);
    return () => { window.removeEventListener('mousedown', swallow); window.removeEventListener('mouseup', release); };
  }, []);
}
