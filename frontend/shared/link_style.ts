/** 캔버스 연결선 모양. 화면 설정과 캔버스가 같은 원본을 본다. */
export type LinkStyle = 'curve' | 'straight' | 'orthogonal';

/** 엔진 preference 키. 모든 창이 snapshot으로 같은 선 모양을 본다. */
export const LINK_STYLE_KEY = 'canvasLinkStyle';
export const LINK_STYLE_OPTIONS: { value: LinkStyle; label: string }[] = [
  { value: 'curve', label: '곡선' }, { value: 'straight', label: '직선' }, { value: 'orthogonal', label: '직각' },
];

export function readLinkStyle(preferences?: Record<string, unknown>): LinkStyle {
  const raw = preferences?.[LINK_STYLE_KEY];
  return LINK_STYLE_OPTIONS.some((option) => option.value === raw) ? raw as LinkStyle : 'curve';
}
