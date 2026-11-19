import { readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

/** 저장소 루트. 정적 검사는 모두 이 기준의 상대 경로로 보고한다. */
export const repo = join(__dirname, '../..');
const skipped = new Set(['dist', 'node_modules']);

/** 폴더 아래에서 이름이 pattern에 맞는 파일을 루트 기준 상대 경로로 모은다. 빌드 산출물은 건너뛴다. */
export function sourceFiles(dir: string, pattern: RegExp): string[] {
  return readdirSync(join(repo, dir)).flatMap((name) => {
    if (skipped.has(name)) return [];
    const path = join(dir, name);
    return statSync(join(repo, path)).isDirectory() ? sourceFiles(path, pattern) : pattern.test(name) ? [path] : [];
  });
}

export const relativeToRepo = (path: string) => relative(repo, path);
