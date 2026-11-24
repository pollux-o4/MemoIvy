import { expect, test, vi } from 'vitest';
import { WindowHeader } from './WindowHeader';
import type { DesktopBridge } from './engine_contract';

test('title double click toggles maximize and excludes interactive controls', () => {
  const windowAction = vi.fn();
  const header = WindowHeader({ title: 'MemoIvy', bridge: { windowAction } as unknown as DesktopBridge, onClose: vi.fn() });
  expect(header.props.onDoubleClick).toBeTypeOf('function');
  for (const interactive of [false, true]) {
    header.props.onDoubleClick({ target: { closest: () => interactive ? {} : null } });
  }
  expect(windowAction.mock.calls).toEqual([['maximize']]);
});

test('title double click works when maximize button is hidden', () => {
  const windowAction = vi.fn();
  const header = WindowHeader({ title: '설정', bridge: { windowAction } as unknown as DesktopBridge, controls: ['minimize'], onClose: vi.fn() });
  header.props.onDoubleClick({ target: { closest: () => null } });
  expect(windowAction).toHaveBeenCalledExactlyOnceWith('maximize');
});
