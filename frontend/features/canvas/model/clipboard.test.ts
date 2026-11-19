import assert from 'node:assert/strict';
import { test } from 'vitest';
import type { Note } from '../../../shared/engine_contract';
import { copyNotes, PASTE_STEP, pastePositions, plainText } from './clipboard';

const note = (id: string, text: string, next: string[] = []): Note => ({ id, text, color: 'yellow', modified: 0, canvas: { position: null, next, done: false } });
const notes = [note('a', 'Alpha', ['b', 'outside']), note('b', 'Beta'), note('c', 'Gamma')];
const positions = { a: { x: 100, y: 200 }, b: { x: 400, y: 260 }, c: { x: 0, y: 0 } };

test('copies selection relative to its top-left and keeps only inner links', () => {
  const payload = copyNotes(['a', 'b'], notes, positions);
  assert.deepEqual(payload?.origin, { x: 100, y: 200 });
  assert.deepEqual(payload?.items.map((item) => item.offset), [{ x: 0, y: 0 }, { x: 300, y: 60 }]);
  assert.deepEqual(payload?.items.map((item) => item.next), [[1], []]);
});

test('each paste round shifts further and keeps the group shape', () => {
  const payload = copyNotes(['a', 'b'], notes, positions);
  assert.ok(payload);
  assert.deepEqual(pastePositions(payload, 2), [{ x: 100 + 2 * PASTE_STEP, y: 200 + 2 * PASTE_STEP }, { x: 400 + 2 * PASTE_STEP, y: 260 + 2 * PASTE_STEP }]);
});

test('empty selection copies nothing and plain text joins bodies', () => {
  assert.equal(copyNotes([], notes, positions), undefined);
  const payload = copyNotes(['a', 'c'], notes, positions);
  assert.ok(payload);
  assert.equal(plainText(payload), 'Alpha\n\nGamma');
});
