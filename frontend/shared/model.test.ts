import { expect, test } from 'vitest';
import { joinTitle, splitTitle } from './model';



test('splitTitle keeps heading marker and separates body; joinTitle round-trips', () => {
  const parts = splitTitle('\n# 프로젝트\n\n소개 첫 줄\n- [ ] 할 일\n');
  expect(parts).toEqual({ marker: '# ', title: '프로젝트', body: '소개 첫 줄\n- [ ] 할 일\n' });
  expect(joinTitle({ ...parts, title: '새 이름' })).toBe('# 새 이름\n\n소개 첫 줄\n- [ ] 할 일');
  expect(joinTitle({ title: '이름만', body: '' })).toBe('이름만');
  expect(splitTitle('')).toEqual({ marker: '', title: '', body: '' });
  expect(joinTitle({ title: '할 일', body: '- [ ] 하나\n  - [ ] \n\n' })).toBe('할 일\n\n- [ ] 하나\n  - [ ] ');
});
