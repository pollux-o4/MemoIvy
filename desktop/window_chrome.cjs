'use strict';

/**
 * 플랫폼별 창 바탕 정책. Linux는 투명 창으로 GTK CSD 둥근 모서리를 그린다(WM 타일링 정상).
 * Windows·macOS는 투명 프레임 없는 창이 Aero Snap(Win+화살표)·최대화에서 빠지므로 불투명 창을 쓴다.
 * 렌더러는 frontend/shared/useWindowFrame.ts의 roundedFrame이 같은 규칙을 따른다.
 */
function windowChrome(platform, dark) {
  if (platform === 'linux') return { transparent: true, backgroundColor: '#00000000' };
  return { transparent: false, backgroundColor: dark ? '#242424' : '#f5f5f5' };
}

module.exports = { windowChrome };
