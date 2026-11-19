import { DesignPage } from '../DesignPage';
import { CHAPTERS } from './chapters';

/** Storybook 첫 페이지. 할 일에 맞는 묶음으로 내려간다. 장 내용은 등록표가 소유한다. */
export function StartHere() {
  return (
    <DesignPage eyebrow="MEMOIVY" title="여기서 시작하세요" lead="할 일을 고르면 갈 곳이 나옵니다. 창 구조와 명세 지도는 그 아래에 있습니다.">
      {CHAPTERS.map((chapter) => (
        <section key={chapter.id} className="dsb-chapter" aria-labelledby={`dsb-${chapter.id}`}>
          <header>
            <h2 id={`dsb-${chapter.id}`}>{chapter.title}</h2>
            <p className="dsb-muted">{chapter.lead}</p>
          </header>
          {chapter.render()}
        </section>
      ))}
    </DesignPage>
  );
}
