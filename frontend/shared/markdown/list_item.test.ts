import { expect, test } from 'vitest';
import { createListItemReader } from './list_item';

test('one blank and content-indented image preserve sequence; two blanks and prose end it', () => {
  const read = createListItemReader();
  const contexts = ['1. first', '', '   ![](x)', '2. second', '', '', '3. separate', 'plain', '4. final']
    .map(text => { read(text); return read.context; });
  expect(contexts[2].owner?.markerLine).toBe(1);
  expect(contexts[3].owner?.sequence).toBe(contexts[0].owner?.sequence);
  expect(contexts[5].owner).toBeNull();
  expect(contexts[6].owner?.sequence).not.toBe(contexts[0].owner?.sequence);
  expect(contexts[7].owner).toBeNull();
});

test('fences pair in order, respecting kind, length, closing suffix and CRLF', () => {
  const read = createListItemReader();
  const contexts = ['````js\r', '```', '~~~~', '```` suffix', '- [ ] code', '````\r', '- [ ] task']
    .map(text => { const item = read(text); return { ...read.context, item }; });
  expect(contexts.map(context => context.fence?.kind)).toEqual(['open', undefined, undefined, undefined, undefined, 'close', undefined]);
  expect(contexts[4].item).toBeUndefined();
  expect(contexts[6].item?.task).toBe(' ');
});

test('nested list continuation returns to its parent by content indentation', () => {
  const read = createListItemReader();
  read('1. parent');
  read('   - child');
  read('   parent continuation');
  expect(read.context.owner?.markerLine).toBe(1);
});
