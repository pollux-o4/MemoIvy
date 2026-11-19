import { childCountLabel } from '../../../../shared/hierarchy';
import type { Progress } from '../../../../shared/progress';
import { Disclosure } from '../../../../shared/Disclosure';
import { TaskCountLabel, TaskProgress } from '../../../../shared/TaskProgress';

/** 프로젝트 카드 아래쪽: 하위 메모 수(제자리 펼치기 단추), 하위 항목 단위 진행과 막대. */
export function ProjectSummary({ label, count, progress, text, peek }: { label: string; count: number; progress: Progress; text: { short: string; title: string }; peek?: { open: boolean; onToggle: () => void; targetId: string } }) {
  return <>
    <div className="canvas-project-meta"><small>{peek && count > 0 && /* biome-ignore lint/a11y/noStaticElementInteractions: 펼침 버튼 누름이 카드 끌기·더블클릭 열기로 번지지 않게 막기만 한다. */ <span className="canvas-peek" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}><Disclosure expanded={peek.open} onToggle={peek.onToggle} targetId={peek.targetId} label={`${label} 하위 메모`}/></span>}<b>{childCountLabel(count)}</b></small>{progress.total > 0 && <small><TaskCountLabel progress={progress} title={text.title}>완료 {text.short}</TaskCountLabel></small>}</div>
    {progress.total > 0 && <TaskProgress className="canvas-project-progress" label={label} progress={progress}/>}
  </>;
}
