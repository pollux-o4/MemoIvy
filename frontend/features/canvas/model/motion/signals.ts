const TRAVEL_SECONDS = 0.65;
export const SIGNAL_SECONDS = 0.95;
export const LINK_SECONDS = 0.28;

function ease(t: number): number {
  const value = Math.max(0, Math.min(t, 1));
  return value * value * (3 - 2 * value);
}

export function pulseSample(elapsed: number, animated: boolean): { travel: number; alpha: number } {
  if (!animated) return { travel: 1, alpha: 1 };
  return {
    travel: ease(elapsed / TRAVEL_SECONDS),
    alpha: 1 - Math.max(0, Math.min((elapsed - TRAVEL_SECONDS) / (SIGNAL_SECONDS - TRAVEL_SECONDS), 1)),
  };
}

export function linkProgress(elapsed: number): number {
  return ease(elapsed / LINK_SECONDS);
}
