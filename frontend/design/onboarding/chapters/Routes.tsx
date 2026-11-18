import { useEffect, useState } from 'react';
import { SECTIONS } from '../../hierarchy';

type Entry = { id: string; title: string; type: string };

/** 묶음마다 사이드바 첫 스토리를 Storybook 색인에서 찾는다. 스토리 id를 따로 적지 않는다. */
function useFirstStories() {
  const [first, setFirst] = useState<Record<string, string>>({});
  useEffect(() => {
    let alive = true;
    fetch('/index.json')
      .then((response) => response.json() as Promise<{ entries: Record<string, Entry> }>)
      .then(({ entries }) => {
        const found: Record<string, string> = {};
        const here = new URLSearchParams(window.location.search).get('id');
        const page = here ? entries[here]?.title : undefined; // 지금 보는 페이지 자신은 갈 곳이 아니다.
        for (const entry of Object.values(entries)) {
          const root = entry.title.split('/')[0];
          if (entry.type === 'story' && entry.title !== page && !found[root]) found[root] = entry.id;
        }
        if (alive) setFirst(found);
      })
      .catch(() => {}); // 색인이 없는 실행(정적 빌드 일부·테스트)에서는 위치만 보여 준다.
    return () => { alive = false; };
  }, []);
  return first;
}

/** 할 일 → 사이드바 묶음 → 코드 위치. */
export function Routes() {
  const first = useFirstStories();
  return (
    <ul className="dsb-grid dsb-routes">
      {SECTIONS.map((section) => (
        <li key={section.root} className="dsb-card" aria-label={section.root}>
          <p>{section.purpose}</p>
          {first[section.root]
            ? <a href={`/?path=/story/${first[section.root]}`} target="_top">{section.root} →</a>
            : <strong>{section.root}</strong>}
          <code>{section.codePath}</code>
        </li>
      ))}
    </ul>
  );
}
