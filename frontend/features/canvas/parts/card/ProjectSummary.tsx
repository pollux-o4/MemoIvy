type Tasks = { total: number; done: number };

/** 프로젝트 카드 아래쪽: 하위 메모 수·할 일 합계와 진행 막대. */
export function ProjectSummary({ count, tasks }: { count: number; tasks: Tasks }) {
  return <>
    <div className="canvas-project-meta"><small><b>메모 {count}</b></small>{tasks.total > 0 && <small>할 일 {tasks.done}/{tasks.total}</small>}</div>
    {tasks.total > 0 && <div className="canvas-project-progress"><span style={{ width: `${100 * tasks.done / tasks.total}%` }}/></div>}
  </>;
}
