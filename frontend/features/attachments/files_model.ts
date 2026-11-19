import type { Asset, Note } from '../../shared/engine_contract';

export function reference(asset: Pick<Asset, 'id'>) { return `![](asset:${asset.id})`; }

export function readableBytes(bytes: number) {
  return bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${Math.round(bytes / 1024)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function daysAgo(at: number | undefined, now = Date.now() / 1000) {
  if (!at) return '오늘';
  const days = Math.floor((now - at) / 86_400);
  return days <= 0 ? '오늘' : days === 1 ? '어제' : `${days}일 전`;
}

export function usersByAsset(notes: Pick<Note, 'id' | 'text' | 'modified'>[]) {
  const users = new Map<string, string[]>();
  for (const note of [...notes].sort((left, right) => right.modified - left.modified)) {
    for (const id of new Set([...note.text.matchAll(/asset:([a-f0-9]{64})/g)].map((match) => match[1]))) {
      users.set(id, [...(users.get(id) ?? []), note.id]);
    }
  }
  return users;
}
