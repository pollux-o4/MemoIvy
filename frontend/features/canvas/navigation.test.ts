import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactElement } from 'react';

const state = vi.hoisted(() => ({ values: [] as unknown[], cursor: 0, bridge: { openWindow: vi.fn(), windowAction: vi.fn() } }));
vi.mock('react', async (original) => ({
  ...await original<typeof import('react')>(),
  useState: (initial: unknown) => {
    const index = state.cursor++;
    if (!(index in state.values)) state.values[index] = typeof initial === 'function' ? initial() : initial;
    return [state.values[index], (value: unknown) => { state.values[index] = value; }];
  },
  useEffect: () => {},
  useEffectEvent: (callback: unknown) => callback,
}));
vi.mock('../../shared/useMemoivy', () => ({ useMemoivy: () => ({ snapshot: {}, bridge: state.bridge }) }));
vi.mock('../../shared/appearance', () => ({ useAppearance: () => {} }));
vi.mock('../../shared/useWindowFrame', () => ({ useWindowFrame: () => '' }));
vi.mock('../../shared/tooltip', () => ({ installTooltips: () => {} }));
vi.mock('../../shared/Icon', () => ({ Icon: () => null }));
vi.mock('../library/Library', () => ({ Library: () => null }));
vi.mock('../editor/Editor', () => ({ Editor: () => null }));
vi.mock('./Canvas', () => ({ Canvas: () => null }));
vi.mock('../attachments/Files', () => ({ Files: () => null }));
vi.mock('../settings/Settings', () => ({ Settings: () => null }));
vi.mock('../help/Help', () => ({ Help: () => null }));

import { App } from '../../app/App';
import { Canvas } from './Canvas';
import { Library } from '../library/Library';

function render(screen: 'library' | 'canvas', desktop = false) {
  state.cursor = 0;
  const root = App({ initialScreen: screen, ...(desktop ? { bridge: state.bridge as never } : {}) });
  return (root.props.children as ReactElement<Record<string, unknown>>[]).filter(Boolean);
}
function open(elements: ReturnType<typeof render>, target: 'library' | 'canvas', id?: string) {
  const component = elements.find((element) => element.type === (target === 'canvas' ? Library : Canvas));
  const callback = component?.props[target === 'canvas' ? 'onOpenCanvas' : 'onOpenLibrary'] as (id?: string) => void;
  callback(id);
}

beforeEach(() => {
  state.values = []; state.cursor = 0;
  vi.resetAllMocks();
  vi.stubGlobal('window', { location: { search: '', protocol: 'http:' } });
});

describe('목록과 캔버스 전환', () => {
  it.each(['library', 'canvas'] as const)('미리보기에서 %s의 반대 화면만 표시한다', (screen) => {
    const target = screen === 'library' ? 'canvas' : 'library';
    open(render(screen), target, target === 'canvas' ? 'project-id' : undefined);
    const elements = render(screen);
    expect(elements.some((element) => element.type === Canvas)).toBe(target === 'canvas');
    expect(elements.some((element) => element.type === Library)).toBe(target === 'library');
    expect(state.values[1]).toBe(target === 'canvas' ? 'project-id' : undefined);
    expect(state.bridge.openWindow).not.toHaveBeenCalled();
  });

  it.each(['library', 'canvas'] as const)('데스크톱에서 %s 창이 열린 뒤 현재 창을 닫는다', async (target) => {
    let resolve!: () => void;
    state.bridge.openWindow.mockReturnValue(new Promise<void>((done) => { resolve = done; }));
    open(render(target === 'canvas' ? 'library' : 'canvas', true), target, target === 'canvas' ? 'project-id' : undefined);
    expect(state.bridge.openWindow).toHaveBeenCalledWith(target, target === 'canvas' ? 'project-id' : undefined);
    expect(state.bridge.windowAction).not.toHaveBeenCalled();
    resolve(); await Promise.resolve();
    expect(state.bridge.windowAction).toHaveBeenCalledExactlyOnceWith('close');
  });

  it('전역 데스크톱 bridge도 같은 전환 규칙을 쓴다', async () => {
    vi.stubGlobal('window', { location: { search: '', protocol: 'file:' }, memoivy: state.bridge });
    state.bridge.openWindow.mockResolvedValue(undefined);
    open(render('library'), 'canvas');
    await Promise.resolve();
    expect(state.bridge.openWindow).toHaveBeenCalledWith('canvas', undefined);
    expect(state.bridge.windowAction).toHaveBeenCalledExactlyOnceWith('close');
  });

  it('목적 창 열기가 실패하면 현재 창을 유지하고 오류를 알린다', async () => {
    state.bridge.openWindow.mockRejectedValue(new Error('failed'));
    open(render('canvas', true), 'library');
    await Promise.resolve();
    expect(state.bridge.windowAction).not.toHaveBeenCalled();
    expect(render('canvas', true).some((element) => element.type === Canvas)).toBe(true);
    expect(state.values[2]).toContain('화면을 열지 못했어요');
    const alert = render('canvas', true).find((element) => element.props.role === 'alert');
    if (!alert) throw new Error('오류 알림이 없음');
    const button = (alert.props.children as ReactElement<{ onClick: () => void }>[]).find((element) => element.type === 'button');
    button?.props.onClick();
    expect(state.values[2]).toBeNull();
  });
});
