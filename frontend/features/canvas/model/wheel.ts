/** GTK src/app/canvas/input/pointer.rs 의 스크롤 규칙. */
const WHEEL_PAN = 60;
const WHEEL_ZOOM = 0.18;
const TOUCHPAD_ZOOM = 0.012;
const LINE_PIXELS = 100 / 3; // Chromium 휠 한 칸 = 100px = 3줄

export type WheelInput = { deltaX: number; deltaY: number; deltaMode: number; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean };
export type WheelIntent =
  | { kind: 'zoom'; factor: number }
  | { kind: 'pan'; delta: [number, number]; smooth: boolean };

/** 마우스 휠은 칸(notch) 단위, 터치패드는 픽셀 단위로 본다. 브라우저는 단위를 알려 주지 않아 크기로 구분한다. */
function notches({ deltaX, deltaY, deltaMode }: WheelInput): [number, number] | null {
  if (deltaMode === 1) return [deltaX / 3, deltaY / 3];
  if (deltaMode === 2) return [deltaX, deltaY];
  // Chromium은 확대 배율 계산 탓에 한 칸을 99.99999…처럼 보낸다.
  const discrete = (value: number) => value === 0 || (Math.abs(value) >= LINE_PIXELS && Math.abs(value - Math.round(value)) < 0.01);
  return discrete(deltaX) && discrete(deltaY) && (deltaX || deltaY) ? [Math.round(deltaX) / 100, Math.round(deltaY) / 100] : null;
}

export function wheelIntent(event: WheelInput): WheelIntent {
  const steps = notches(event);
  if (event.ctrlKey || event.metaKey) {
    const factor = steps ? Math.exp(-steps[1] * WHEEL_ZOOM) : Math.exp(-event.deltaY * TOUCHPAD_ZOOM);
    return { kind: 'zoom', factor };
  }
  if (!steps) return { kind: 'pan', delta: [event.deltaX, event.deltaY], smooth: false };
  // Shift+휠: 백엔드가 이미 가로축으로 보냈어도 같은 가로 이동으로 유지한다.
  const [dx, dy] = event.shiftKey ? [steps[0] + steps[1], 0] : steps;
  return { kind: 'pan', delta: [dx * WHEEL_PAN, dy * WHEEL_PAN], smooth: true };
}
