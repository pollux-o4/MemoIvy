import { describe, expect, it } from 'vitest';
import { readSpecAreas } from './readSpecs';

describe('readSpecAreas', () => {
  it('종류/영역 두 단계로 묶고 ID 순으로 정렬한다', () => {
    const areas = readSpecAreas({
      '../docs/specs/behavior/notes/editing/FMT-004--images.md': '# FMT-004 · 이미지\n',
      '../docs/specs/behavior/notes/NOTE-001--x.md': '# NOTE-001 · 편집 창\n',
      '../docs/specs/data/DATA-001--x.md': '# DATA-001 · 메모 파일\n',
      '../docs/specs/data/README.md': '목차\n',
    });
    expect(areas).toEqual([
      { area: 'behavior/notes', specs: [{ id: 'FMT-004', title: '이미지' }, { id: 'NOTE-001', title: '편집 창' }] },
      { area: 'data', specs: [{ id: 'DATA-001', title: '메모 파일' }] },
    ]);
  });
});
