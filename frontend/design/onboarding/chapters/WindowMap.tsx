import windowKinds from '../../../../desktop/window_kinds.json';

const SCALE = 0.06; // 가장 큰 창(1000px)이 60px 높이 안에 들어가는 비율

/** Electron 창 종류(desktop/window_kinds.json)에서 그린 창 지도. 크기를 여기 다시 적지 않는다. */
export function WindowMap() {
  return (
    <ul className="dsb-grid">
      {Object.entries(windowKinds).map(([kind, size]) => (
        <li key={kind} className="dsb-card" aria-label={kind}>
          <div className="dsb-window" aria-hidden="true">
            <span style={{ width: size.width * SCALE, height: size.height * SCALE }}/>
          </div>
          <h3>{kind}</h3>
          <ul>
            <li>기본<span className="dsb-value">{size.width} × {size.height}</span></li>
            <li>최소<span className="dsb-value">{size.minWidth} × {size.minHeight}</span></li>
          </ul>
        </li>
      ))}
    </ul>
  );
}
