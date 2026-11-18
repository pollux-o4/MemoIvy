interface Spec { id: string; title: string }
export interface SpecArea { area: string; specs: Spec[] }

/** `docs/specs/<종류>/<영역>/…` 경로와 첫 줄 `# ID · 제목`에서 명세 지도를 만든다. 명세 목록을 여기 다시 적지 않는다. */
export function readSpecAreas(files: Record<string, string>): SpecArea[] {
  const areas = new Map<string, Spec[]>();
  for (const [path, markdown] of Object.entries(files)) {
    const heading = markdown.match(/^# (\S+) · (.+)$/m);
    const folders = path.split('docs/specs/')[1]?.split('/').slice(0, -1);
    if (!heading || !folders?.length) continue;
    const area = folders.slice(0, 2).join('/');
    areas.set(area, [...(areas.get(area) ?? []), { id: heading[1], title: heading[2] }]);
  }
  return [...areas]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([area, specs]) => ({ area, specs: specs.sort((a, b) => a.id.localeCompare(b.id)) }));
}
