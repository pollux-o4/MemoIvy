import { Icon } from '../../../shared/Icon';

/** 빈 캔버스 안내와 첫 메모 만들기. */
export function CanvasEmpty({ onCreate }: { onCreate: () => void }) {
  return <div className="canvas-empty">
    <div className="empty-mark"><Icon name="canvas"/></div>
    <h2>이 캔버스는 비어 있어요</h2>
    <p>새 메모를 만들거나 라이브러리에서 메모를 추가해 보세요.</p>
    <button className="primary-button" onClick={onCreate}><Icon name="plus"/> 새 메모</button>
  </div>;
}
