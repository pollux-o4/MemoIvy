import type { CSSProperties, ReactNode } from 'react';
import { projectSheets } from '../../shared/model';


// 프로젝트 = 종이 더미. 접힘에서 하위 메모 수만큼(1~2장) 뒤 종이가 오른쪽 아래로 비껴 쌓이고, 펼치면 풀린다.
export function ProjectStack({ childCount, expanded, nested, accent, children }: { childCount: number; expanded: boolean; nested: boolean; accent: string; children: ReactNode }) {
  const sheets = projectSheets(childCount);
  return <div className={`project-stack${expanded ? ' expanded' : ''}${nested ? ' nested' : ''}`} data-sheets={sheets} style={{ '--note-accent': accent } as CSSProperties}>
    {sheets > 1 && <span className="project-sheet second" aria-hidden="true"/>}
    <span className="project-sheet first" aria-hidden="true"/>
    {children}
  </div>;
}
