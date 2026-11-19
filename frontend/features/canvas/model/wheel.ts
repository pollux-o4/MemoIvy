/**
 * 휠 입력 해석: 장치 판별(classifyWheel) → 의도 변환(wheelIntent).
 * 마우스 휠은 칸 단위로 일정 거리를 부드럽게, 터치패드는 손가락을 1:1로 따르되 관성 튐을 줄인다.
 */

/** 휠 한 칸 이동 거리(화면 px). Figma·Miro 수준(약 50px)에 맞춰 GTK의 60보다 줄였다. */
export const WHEEL_PAN = 48;
/** 한 이벤트에 반영하는 터치패드 이동의 상한(화면 px). OS 관성(fling) 끝의 튀는 값을 누른다. */
export const TOUCHPAD_MAX_STEP = 40;
const WHEEL_ZOOM = 0.18;
const TOUCHPAD_ZOOM = 0.012;
/** 브라우저 한 칸 단위: Windows·macOS Chromium 100px, Linux Chromium 53⅓px(×화면 배율), Firefox 줄 모드 3줄. */
const NOTCH_UNITS = [100, 160 / 3, 120];
/** 이 값 이상이면서 칸 단위의 배수면 휠 칸으로 본다. 터치패드 한 이벤트는 보통 이보다 작다. */
const MIN_NOTCH_PIXELS = 30;

export type WheelInput = { deltaX: number; deltaY: number; deltaMode: number; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean };
export type WheelIntent =
  | { kind: 'zoom'; factor: number }
  | { kind: 'pan'; delta: [number, number]; smooth: boolean };
type WheelSource = { device: 'wheel'; notches: [number, number] } | { device: 'touchpad'; pixels: [number, number] };

function notchCount(value: number, scale: number): number | null {
  if (value === 0) return 0;
  if (Math.abs(value) < MIN_NOTCH_PIXELS * scale) return null;
  for (const unit of NOTCH_UNITS) {
    const count = value / (unit * scale);
    if (Math.abs(count - Math.round(count)) < 0.01) return Math.round(count);
  }
  return null;
}

/** 장치 판별. 브라우저가 장치를 알려 주지 않아 단위·크기로 구분한다. scale은 화면 배율(devicePixelRatio). */
export function classifyWheel({ deltaX, deltaY, deltaMode }: WheelInput, scale = 1): WheelSource {
  if (deltaMode === 1) return { device: 'wheel', notches: [deltaX / 3, deltaY / 3] };
  if (deltaMode === 2) return { device: 'wheel', notches: [deltaX, deltaY] };
  for (const factor of scale === 1 ? [1] : [1, scale]) {
    const x = notchCount(deltaX, factor); const y = notchCount(deltaY, factor);
    if (x !== null && y !== null && (x || y)) return { device: 'wheel', notches: [x, y] };
  }
  return { device: 'touchpad', pixels: [deltaX, deltaY] };
}

const clampStep = (value: number) => Math.max(-TOUCHPAD_MAX_STEP, Math.min(TOUCHPAD_MAX_STEP, value));

export function wheelIntent(event: WheelInput, scale = 1): WheelIntent {
  const source = classifyWheel(event, scale);
  if (event.ctrlKey || event.metaKey) {
    return { kind: 'zoom', factor: source.device === 'wheel' ? Math.exp(-source.notches[1] * WHEEL_ZOOM) : Math.exp(-event.deltaY * TOUCHPAD_ZOOM) };
  }
  if (source.device === 'touchpad') return { kind: 'pan', delta: [clampStep(source.pixels[0]), clampStep(source.pixels[1])], smooth: false };
  // Shift+휠: 백엔드가 이미 가로축으로 보냈어도 같은 가로 이동으로 유지한다.
  const [dx, dy] = event.shiftKey ? [source.notches[0] + source.notches[1], 0] : source.notches;
  return { kind: 'pan', delta: [dx * WHEEL_PAN, dy * WHEEL_PAN], smooth: true };
}
