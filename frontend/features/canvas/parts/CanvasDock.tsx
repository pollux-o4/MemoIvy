import { Icon } from '../../../shared/Icon';
import { useSaveLabel } from '../../../shared/useSaveLabel';

export function CanvasDock({ count, links, zoom, saving, error, canUndo, canRedo, onUndo, onRedo, onZoom, onFit, onReset, onHelp }: {
  count: number; links: number; zoom: number; saving: boolean; error: unknown; canUndo: boolean; canRedo: boolean;
  onUndo: () => void; onRedo: () => void; onZoom: (direction: -1 | 1) => void; onFit: () => void; onReset: () => void; onHelp: () => void;
}) {
  const saveLabel = useSaveLabel(saving);
  const status = error ? '저장 실패' : saveLabel;
  return <footer className="canvas-dock">
    <button aria-label="되돌리기" title="되돌리기 · Ctrl+Z" onClick={onUndo} disabled={!canUndo}><Icon name="undo"/></button>
    <button aria-label="다시 실행" title="다시 실행 · Ctrl+Shift+Z" onClick={onRedo} disabled={!canRedo}><Icon name="redo"/></button>
    <i/>
    <button aria-label="축소" title="축소 · Ctrl+-" onClick={() => onZoom(-1)}><Icon name="zoomOut"/></button>
    <button className="zoom-value" title="100%로 보기 · Ctrl+0" aria-label="100%로 보기" onClick={onReset}>{Math.round(zoom * 100)}%</button>
    <button aria-label="확대" title="확대 · Ctrl++" onClick={() => onZoom(1)}><Icon name="zoomIn"/></button>
    <button className="canvas-fit-button" title="전체 보기 · Shift+1" aria-label="전체 보기" onClick={onFit}><Icon name="fit"/></button>
    <i/>
    <button className="canvas-help-button" title="캔버스 사용법" aria-label="캔버스 사용법" onClick={onHelp}><Icon name="help"/></button>
    <span className="canvas-summary">{`메모 ${count} · 연결 ${links}`}</span>
    <span className={`canvas-status${error ? ' failed' : ''}`}>{status}</span>
  </footer>;
}
