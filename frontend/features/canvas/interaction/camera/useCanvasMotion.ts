import { onMotionPreferenceChange, prefersReducedMotion } from '../../../../shared/appearance';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';
import { CameraMotion, Coast, ZOOM_STEP, autopan, firstLayout, limitLead, LINK_SECONDS, linkProgress, panned, pulseSample, SIGNAL_SECONDS, zoomAt } from '../../model/motion';
import type { Camera, Rect } from '../../model/motion';
import { WHEEL_PAN } from '../../model/wheel';

type LinkSample = { progress: number; travel: number; alpha: number; creating: boolean; pulsing: boolean };
/** 휠을 빠르게 굴려도 화면이 따라잡을 거리. 약 네 칸(WHEEL_PAN×4)을 넘으면 더 쌓지 않는다. */
const MAX_WHEEL_LEAD = WHEEL_PAN * 4;
const restingLink: LinkSample = { progress: 1, travel: 1, alpha: 1, creating: false, pulsing: false };

export function useCanvasMotion(host: RefObject<HTMLDivElement | null>, cards: Rect[], selectedCards: Rect[]) {
  const [camera, setCameraState] = useState<Camera>({ x: 0, y: 0, zoom: 1 });
  const [linkSamples, setLinkSamples] = useState<Record<string, LinkSample>>({});
  const [animated, setAnimated] = useState(() => typeof window === 'undefined' || !prefersReducedMotion());
  const cameraRef = useRef(camera);
  const motionRef = useRef<CameraMotion | null>(null);
  const coastRef = useRef<Coast | null>(null);
  const autopanRef = useRef<[number, number] | null>(null);
  const linkStarts = useRef(new Map<string, { at: number; creating: boolean }>());
  const raf = useRef<number | null>(null);
  const frameRef = useRef<(now: number) => void>(() => undefined);
  const lastFrame = useRef<number | null>(null);
  const viewport = useRef<[number, number]>([0, 0]);
  const didFirstLayout = useRef(false);
  const latestLayout = useRef({ cards, selectedCards });
  latestLayout.current = { cards, selectedCards };

  const pendingZoom = useRef(false);
  const publish = useCallback((next: Camera) => {
    pendingZoom.current = false;
    cameraRef.current = next;
    if (motionRef.current) motionRef.current.current = next;
    setCameraState(next);
  }, []);

  const schedule = useCallback(() => {
    if (raf.current === null) raf.current = requestAnimationFrame((now) => frameRef.current(now));
  }, []);

  const frame = useCallback((now: number) => {
    raf.current = null;
    if (pendingZoom.current) publish(cameraRef.current);
    const elapsed = lastFrame.current === null ? 0 : Math.min((now - lastFrame.current) / 1000, 0.05);
    lastFrame.current = now;
    const currentMotion = motionRef.current;
    if (currentMotion && !currentMotion.settled) publish(currentMotion.tick(elapsed));
    if (coastRef.current) {
      const result = coastRef.current.advance(elapsed);
      coastRef.current = result.next;
      if (result.delta[0] || result.delta[1]) {
        const current = cameraRef.current;
        publish({ ...current, x: current.x - result.delta[0] / current.zoom, y: current.y - result.delta[1] / current.zoom });
      }
    }
    if (autopanRef.current) {
      const [dx, dy] = autopan(autopanRef.current, viewport.current, elapsed);
      if (dx || dy) {
        const current = cameraRef.current;
        publish({ ...current, x: current.x + dx / current.zoom, y: current.y + dy / current.zoom });
      }
    }
    // 끝난 신호는 기록에서 지운다: 표본 맵이 진행 중인 간선 수로 묶이고, 쉬는 간선은 restingLink 고정 참조를 받아 다시 그려지지 않는다.
    const samples: Record<string, LinkSample> = {};
    let linksActive = false;
    const hadLinks = linkStarts.current.size > 0;
    linkStarts.current.forEach(({ at, creating }, key) => {
      const seconds = (now - at) / 1000;
      if (!animated || seconds >= SIGNAL_SECONDS) { linkStarts.current.delete(key); return; }
      const pulse = pulseSample(seconds, animated);
      samples[key] = { progress: creating ? linkProgress(seconds) : 1, travel: pulse.travel, alpha: pulse.alpha, creating: creating && seconds < LINK_SECONDS, pulsing: true };
      linksActive = true;
    });
    if (hadLinks) setLinkSamples(samples);
    if ((currentMotion && !currentMotion.settled) || coastRef.current || autopanRef.current || linksActive) schedule();
    else lastFrame.current = null;
  }, [animated, publish, schedule]);
  frameRef.current = frame;

  useEffect(() => {
    return onMotionPreferenceChange(() => setAnimated(!prefersReducedMotion()));
  }, []);

  useEffect(() => {
    if (animated) return;
    coastRef.current = null;
    linkStarts.current.clear();
    setLinkSamples({});
    const current = cameraRef.current;
    motionRef.current?.to(current, false);
    if (pendingZoom.current) publish(current);
    if (raf.current !== null) cancelAnimationFrame(raf.current);
    raf.current = null;
  }, [animated, publish]);

  useLayoutEffect(() => {
    const board = host.current;
    if (!board) return;
    const layout = () => {
      const rect = board.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) return;
      const resized = rect.width !== viewport.current[0] || rect.height !== viewport.current[1];
      viewport.current = [rect.width, rect.height];
      if (!motionRef.current || resized) {
        motionRef.current = new CameraMotion(viewport.current);
        motionRef.current.current = cameraRef.current;
      }
      if (didFirstLayout.current || !latestLayout.current.cards.length) return;
      if (!latestLayout.current.selectedCards.length) { didFirstLayout.current = true; return; }
      const initial = firstLayout(latestLayout.current.cards, latestLayout.current.selectedCards, viewport.current);
      if (initial) {
        didFirstLayout.current = true;
        motionRef.current.to(initial, false);
        publish(initial);
      }
    };
    const observer = new ResizeObserver(layout);
    observer.observe(board);
    layout();
    return () => observer.disconnect();
  }, [host, publish]);

  useLayoutEffect(() => {
    if (didFirstLayout.current || viewport.current[0] <= 0 || viewport.current[1] <= 0 || !cards.length) return;
    didFirstLayout.current = true;
    if (!selectedCards.length) return;
    const initial = firstLayout(cards, selectedCards, viewport.current);
    if (!initial) return;
    motionRef.current?.to(initial, false);
    publish(initial);
  }, [cards, selectedCards, publish]);

  // 취소한 예약은 ref도 비운다: StrictMode 재마운트처럼 ref가 살아남으면 schedule()이 영영 막힌다.
  useEffect(() => () => {
    if (raf.current !== null) cancelAnimationFrame(raf.current);
    raf.current = null;
    lastFrame.current = null;
    pendingZoom.current = false;
  }, []);

  const setCamera = useCallback((next: Camera | ((current: Camera) => Camera)) => {
    const value = typeof next === 'function' ? next(cameraRef.current) : next;
    motionRef.current?.to(value, false);
    publish(value);
  }, [publish]);
  const animateTo = useCallback((target: Camera) => {
    const board = host.current;
    if (!board) return;
    const rect = board.getBoundingClientRect();
    viewport.current = [rect.width, rect.height];
    if (!motionRef.current) motionRef.current = new CameraMotion(viewport.current);
    motionRef.current.current = cameraRef.current;
    motionRef.current.to(target, animated);
    if (!animated) publish(motionRef.current.current);
    else schedule();
  }, [animated, host, publish, schedule]);
  const motionTarget = useCallback(() => motionRef.current?.target ?? cameraRef.current, []);
  const zoomByStep = useCallback((direction: 1 | -1) => {
    const [width, height] = viewport.current;
    animateTo(zoomAt(cameraRef.current, [width / 2, height / 2], motionTarget().zoom * (direction > 0 ? ZOOM_STEP : 1 / ZOOM_STEP)));
  }, [animateTo, motionTarget]);
  const zoomAtScreen = useCallback((point: [number, number], factor: number) => {
    coastRef.current = null;
    const current = cameraRef.current;
    const next = zoomAt(current, point, current.zoom * factor);
    motionRef.current?.to(next, false);
    if (next === current) return;
    // 포인터 기준 좌표는 입력마다 계산하고 화면에는 프레임당 한 번만 반영한다.
    cameraRef.current = next;
    pendingZoom.current = true;
    schedule();
  }, [schedule]);
  /** 화면 거리만큼 이동. animated면 진행 중인 이동 목표에 이어 붙인다(GTK wheel_pan). */
  const panBy = useCallback((delta: [number, number], smooth: boolean) => {
    coastRef.current = null;
    if (smooth) animateTo(limitLead(cameraRef.current, panned(motionTarget(), delta), MAX_WHEEL_LEAD));
    else setCamera(panned(cameraRef.current, delta));
  }, [animateTo, motionTarget, setCamera]);
  const startCoast = useCallback((velocity: [number, number]) => {
    coastRef.current = animated ? Coast.start(velocity) : null;
    if (coastRef.current) schedule();
  }, [animated, schedule]);
  const setAutopan = useCallback((screen: [number, number] | null) => {
    autopanRef.current = screen;
    if (screen) schedule();
  }, [schedule]);
  const triggerLink = useCallback((key: string, creating: boolean) => {
    if (!animated) { setLinkSamples((current) => ({ ...current, [key]: restingLink })); return; }
    linkStarts.current.set(key, { at: performance.now(), creating });
    schedule();
  }, [animated, schedule]);
  const sampleForLink = useCallback((key: string) => linkSamples[key] ?? restingLink, [linkSamples]);

  return { camera, cameraRef, setCamera, animateTo, zoomByStep, zoomAtScreen, panBy, startCoast, setAutopan, triggerLink, sampleForLink, animated };
}
