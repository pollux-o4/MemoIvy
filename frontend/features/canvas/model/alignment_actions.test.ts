import { describe, expect, it } from 'vitest';
import { ALIGN_GROUPS, alignmentForKey, chordLabel } from './alignment_actions';

const press = (key: string, shiftKey = false) => alignmentForKey({ key, altKey: true, shiftKey, ctrlKey: false, metaKey: false });

describe('alignment actions', () => {
  it('maps CANVAS-003 chords to alignments', () => {
    expect([press('a'), press('H'), press('h', true), press('V', true), press('f')]).toEqual(['left', 'center', 'distribute-x', 'distribute-y', 'flow']);
    expect(alignmentForKey({ key: 'a', altKey: true, shiftKey: false, ctrlKey: true, metaKey: false })).toBeUndefined();
  });

  it('chords are unique and labelled for tooltips', () => {
    const labels = ALIGN_GROUPS.flatMap((group) => group.rows.flat().map((item) => chordLabel(item.chord)));
    expect(new Set(labels).size).toBe(labels.length);
    expect(labels).toContain('Alt+Shift+H');
  });
});
