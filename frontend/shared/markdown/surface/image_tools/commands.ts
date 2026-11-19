import { parseListItem } from '../../list_item';

const MIN_IMAGE_WIDTH = 64;
export const MAX_IMAGE_WIDTH = 4096;

export function displayImageSize(width: number | undefined, natural: { width: number; height: number }, available: number) {
  const originalWidth = Math.max(1, natural.width);
  const originalHeight = Math.max(1, natural.height);
  const min = Math.min(MIN_IMAGE_WIDTH, Math.max(0, available));
  const shownWidth = Math.max(min, Math.min(width ?? originalWidth, Math.max(min, available)));
  return { width: shownWidth, height: Math.round(shownWidth * originalHeight / originalWidth) };
}

export function resizeImageWidth(start: number, dx: number, dy: number, ratio: number, available: number) {
  const min = Math.min(MIN_IMAGE_WIDTH, Math.max(0, available));
  const delta = (dx + dy / Math.max(0.01, ratio)) / 2;
  return Math.max(min, Math.min(Math.round(start + delta), Math.max(min, available)));
}

export function imageMarkup(description: string, width: number | undefined, hash: string) {
  return `![${description}${width ? `|${width}` : ''}](asset:${hash})`;
}

export function deleteImageLine(text: string, from: number, to: number) {
  const start = Math.max(0, Math.min(from, text.length));
  const end = Math.max(start, Math.min(to, text.length));
  if (end < text.length) return text.slice(0, start) + text.slice(end + 1);
  if (start > 0) return text.slice(0, start - 1) + text.slice(end);
  return text.slice(end);
}

export function imageDropIndent(target: string, after: boolean) {
  return after ? (parseListItem(target)?.contentIndent ?? 0) : 0;
}

export function moveImageLine(text: string, sourceFrom: number, targetFrom: number, after = false) {
  const lines = text.split('\n');
  const starts: number[] = [];
  let offset = 0;
  for (const line of lines) {
    starts.push(offset);
    offset += line.length + 1;
  }
  const source = starts.indexOf(sourceFrom);
  const target = starts.indexOf(targetFrom);
  if (source < 0 || target < 0 || source === target) return text;
  const indent = imageDropIndent(lines[target], after);
  const [image] = lines.splice(source, 1);
  const moved = `${' '.repeat(indent)}${image.trimStart()}`;
  const insertion = target + Number(after) - Number(target > source);
  lines.splice(insertion, 0, moved);
  return lines.join('\n');
}
