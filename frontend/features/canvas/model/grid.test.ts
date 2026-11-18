import { describe, expect, it } from 'vitest';
import { gridStyle } from './grid';

describe('gridStyle', () => {
  it('preserves the canvas grid calculation and CSS values', () => {
    expect(gridStyle({ x: 13, y: -29, zoom: 0.75 })).toEqual({
      '--grid-size': '36px',
      '--grid-gap': '18px',
      '--grid-x': '26.25px',
      '--grid-y': '21.75px',
      '--grid-minor-opacity': '12.5%',
    });
  });

  it('keeps offsets positive for negative camera coordinates', () => {
    expect(gridStyle({ x: -13, y: -29, zoom: 1 })).toMatchObject({
      '--grid-x': '13px',
      '--grid-y': '29px',
    });
  });

  it('changes grid scale at zoom thresholds', () => {
    expect(gridStyle({ x: 0, y: 0, zoom: 0.66 })).toMatchObject({
      '--grid-gap': '31.68px',
      '--grid-size': '63.36px',
    });
    expect(gridStyle({ x: 0, y: 0, zoom: 2 / 3 })).toMatchObject({
      '--grid-gap': `${24 * (2 / 3)}px`,
      '--grid-size': `${48 * (2 / 3)}px`,
    });
  });
});
