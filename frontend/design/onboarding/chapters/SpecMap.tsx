import { readSpecAreas } from '../readSpecs';

const files = import.meta.glob<string>('../../../../docs/specs/**/*.md', { query: '?raw', import: 'default', eager: true });

/** 동작 계약의 원본은 docs/specs다. ID로 검색해 원문을 읽는다. */
export function SpecMap() {
  return (
    <ul className="dsb-grid">
      {readSpecAreas(files).map(({ area, specs }) => (
        <li key={area} className="dsb-card" aria-label={area}>
          <h3>{area}</h3>
          <ul>
            {specs.map((spec) => (
              <li key={spec.id}><span className="dsb-id">{spec.id}</span>{spec.title}</li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}
