import { describe, expect, it } from 'vitest';
import { engineError } from './desktop_errors';

describe('engineError', () => {
  it('restores code and snapshot from the preload envelope', () => {
    const error = engineError({ __memoivyError: { message: 'm', code: 'CONFLICT', snapshot: { revision: 3 } } }) as Error & { code: string; snapshot: { revision: number } };
    expect([error instanceof Error, error.message, error.code, error.snapshot.revision]).toEqual([true, 'm', 'CONFLICT', 3]);
  });
  it('passes other reasons through', () => { const e = new Error('x'); expect(engineError(e)).toBe(e); });
});
