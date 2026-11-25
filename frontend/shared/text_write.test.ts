import { describe, expect, it } from 'vitest';
import type { Snapshot } from './engine_contract';
import { isConflict, rebaseRevision, TEXT_CONFLICT_MESSAGE, userMessage } from './text_write';

const latest = (text: string) => ({ revision: 9, notes: [{ id: 'n', text }] }) as unknown as Snapshot;

describe('text_write', () => {
  it('rebases when only non-text fields changed', () => expect(rebaseRevision(latest('a'), 'n', 'a')).toBe(9));
  it('keeps conflict when text changed elsewhere', () => expect(rebaseRevision(latest('b'), 'n', 'a')).toBeUndefined());
  it('detects conflict code only', () => expect([isConflict({ code: 'CONFLICT' }), isConflict(new Error('x'))]).toEqual([true, false]));
  it('maps the engine conflict message', () => expect(userMessage('This note changed in another window.')).toBe(TEXT_CONFLICT_MESSAGE));
});
