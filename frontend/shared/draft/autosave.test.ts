import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { createAutosave } from './autosave';

const policy = { idle: 300, maxWait: 1000 };

describe('createAutosave', () => {
  beforeEach(() => { vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  test('입력이 멈추면 마지막 값만 한 번 저장한다', () => {
    const save = vi.fn();
    const auto = createAutosave(policy, save);
    auto.schedule('a'); vi.advanceTimersByTime(100); auto.schedule('ab');
    vi.advanceTimersByTime(299); expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1); expect(save).toHaveBeenCalledExactlyOnceWith('ab');
  });

  test('계속 입력해도 maxWait 안에 저장한다', () => {
    const save = vi.fn();
    const auto = createAutosave(policy, save);
    for (let t = 0; t < 1000; t += 200) { auto.schedule(String(t)); vi.advanceTimersByTime(200); }
    expect(save).toHaveBeenCalledExactlyOnceWith('800');
  });

  test('flush는 기다리던 값만 즉시 저장하고 cancel은 버린다', () => {
    const save = vi.fn();
    const auto = createAutosave(policy, save);
    auto.flush(); expect(save).not.toHaveBeenCalled();
    auto.schedule('x'); auto.flush(); expect(save).toHaveBeenCalledExactlyOnceWith('x');
    auto.schedule('y'); auto.cancel(); vi.runAllTimers(); expect(save).toHaveBeenCalledTimes(1);
  });
});
