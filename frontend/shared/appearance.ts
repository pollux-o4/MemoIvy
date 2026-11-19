import { useEffect } from 'react';

/** 엔진 preference 키 `appearance`에 저장해 모든 창이 snapshot으로 공유한다. */
export type ThemeChoice = 'system' | 'light' | 'dark';
export type MotionChoice = 'system' | 'reduce' | 'full';
export type AppearancePreference = { theme: ThemeChoice; motion: MotionChoice };
export const APPEARANCE_KEY = 'appearance';

export function readAppearance(preferences?: Record<string, unknown>): AppearancePreference {
  const raw = preferences?.[APPEARANCE_KEY] as Partial<AppearancePreference> | undefined;
  const theme: ThemeChoice = raw?.theme === 'light' || raw?.theme === 'dark' ? raw.theme : 'system';
  const motion: MotionChoice = raw?.motion === 'reduce' || raw?.motion === 'full' ? raw.motion : 'system';
  return { theme, motion };
}

/** 선택을 <html>에 반영한다. system이면 표시를 지워 운영체제 미디어 쿼리를 따른다. */
function applyAppearance({ theme, motion }: AppearancePreference, root: HTMLElement = document.documentElement) {
  root.classList.toggle('theme-light', theme === 'light');
  root.classList.toggle('theme-dark', theme === 'dark');
  if (motion === 'system') delete root.dataset.motion; else root.dataset.motion = motion;
  root.dispatchEvent(new CustomEvent('memoivy:motion-change'));
}

export function useAppearance(preferences?: Record<string, unknown>) {
  const { theme, motion } = readAppearance(preferences);
  useEffect(() => { applyAppearance({ theme, motion }); }, [theme, motion]);
}

/** 캔버스 등 스크립트 모션의 단일 판단 기준. 앱 설정이 운영체제 설정보다 우선한다. */
export function prefersReducedMotion(): boolean {
  const choice = document.documentElement.dataset.motion;
  if (choice === 'reduce') return true;
  if (choice === 'full') return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** prefersReducedMotion 값이 바뀔 수 있는 모든 경로를 구독한다. */
export function onMotionPreferenceChange(listener: () => void): () => void {
  const query = window.matchMedia('(prefers-reduced-motion: reduce)');
  query.addEventListener('change', listener);
  document.documentElement.addEventListener('memoivy:motion-change', listener);
  return () => { query.removeEventListener('change', listener); document.documentElement.removeEventListener('memoivy:motion-change', listener); };
}
