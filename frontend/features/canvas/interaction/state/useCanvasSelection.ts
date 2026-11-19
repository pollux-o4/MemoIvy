import { useState } from 'react';

export type CanvasPoint = { x: number; y: number };
export type CanvasMode = 'idle' | 'pan' | 'cards' | 'marquee' | 'connect';

export function useCanvasSelection() {
  const [selected, setSelected] = useState<string[]>([]);
  const [mode, setMode] = useState<CanvasMode>('idle');
  const [marquee, setMarquee] = useState<{ a: CanvasPoint; b: CanvasPoint } | null>(null);
  const [connectFrom, setConnectFrom] = useState<string | null>(null);
  return { selected, setSelected, mode, setMode, marquee, setMarquee, connectFrom, setConnectFrom };
}
