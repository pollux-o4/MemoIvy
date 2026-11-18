import { EditorState } from '@codemirror/state';
import { expect, test } from 'vitest';
import { createListScanner, nextListMarker, parseListItem } from './list_markers';

const scan = (doc: string) => {
  const state = EditorState.create({ doc });
  const next = createListScanner();
  return Array.from({ length: state.doc.lines }, (_, i) => {
    const line = state.doc.line(i + 1);
    return next(line).map((range) => [range.from - line.from, range.to - line.from]);
  });
};

test('list markers are hidden on item lines', () => {
  expect(scan('- 항목\n- [ ] 할 일')).toEqual([[[0, 0], [0, 2]], [[0, 0], [0, 6]]]);
});

test('lines indented by the marker width continue the item and hide their indent', () => {
  expect(scan('- 시프트로 바로\n  여기서 이어짐\n본문')).toEqual([[[0, 0], [0, 2]], [[0, 0], [0, 2]], []]);
});

test('a line indented less than the marker width is not a continuation', () => {
  expect(scan('- [ ] 할 일\n  덜 들여씀')).toEqual([[[0, 0], [0, 6]], []]);
});

test('ordered list markers are displayed and continue their numbering', () => {
  const item = parseListItem('  9) 다음 항목');
  expect(item?.marker).toBe('  9) ');
  expect(nextListMarker(item!)).toBe('  10) ');
});

test('a list prefix requires its separating space', () => {
  expect(parseListItem('-5도')).toBeNull();
  expect(parseListItem('- 항목')?.marker).toBe('- ');
});

test('list markers are ignored in fenced code and blockquotes', () => {
  expect(scan('```md\n- 코드 안\n```\n> - 인용 안\n- 목록')).toEqual([[], [], [], [], [[0, 0], [0, 2]]]);
});
