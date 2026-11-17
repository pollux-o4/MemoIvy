import { useMemo, useRef, useState } from 'react';
import './canvas.css';
import type { Note } from '../../shared/engine_contract';
import { Icon } from '../../shared/Icon';
import { checklist, plainText, titleOf } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';

type Point = { x: number; y: number };
const cardWidth = 240;
const cardHeight = 156;
const colors: Record<string, string> = { yellow: 'var(--note-yellow)', blue: 'var(--note-blue)', green: 'var(--note-green)', pink: 'var(--note-pink)', purple: 'var(--note-purple)' };

export function Canvas({ store, scope, onOpenNote, onBack }: { store: MemoStore; scope?: string; onOpenNote: (id: string) => void; onBack: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const [camera, setCamera] = useState({ x: 0, y: 0, zoom: 1 });
  const [selected, setSelected] = useState<string[]>([]);
  const [mode, setMode] = useState<'idle' | 'pan' | 'cards' | 'marquee' | 'connect'>('idle');
  const [drag, setDrag] = useState<{ start: Point; origin: Point; positions: Record<string, Point>; shift: boolean } | null>(null);
  const [marquee, setMarquee] = useState<{ a: Point; b: Point } | null>(null);
  const [connectFrom, setConnectFrom] = useState<string | null>(null);
  const [history, setHistory] = useState<Array<Record<string, Point>>>([]);
  const [redo, setRedo] = useState<Array<Record<string, Point>>>([]);
  const [draftPositions, setDraftPositions] = useState<Record<string, Point>>({});
  const spaceDown = useRef(false);
  const notes = useMemo(() => store.notes.filter((note) => (note.canvas.parent ?? null) === (scope ?? null)), [store.notes, scope]);
  const positions = useMemo(() => Object.fromEntries(notes.map((note, index) => [note.id, draftPositions[note.id] ?? (note.canvas.position ? { x: note.canvas.position[0], y: note.canvas.position[1] } : { x: 90 + (index % 3) * 310, y: 100 + Math.floor(index / 3) * 240 })])), [notes, draftPositions]);
  const scopedProject = store.notes.find((note) => note.id === scope);
  const title = scopedProject ? titleOf(scopedProject) : '메모 캔버스';
  const noteById = new Map(notes.map((note) => [note.id, note]));
  const screenPoint = (event: { clientX: number; clientY: number }): Point => { const rect = host.current?.getBoundingClientRect(); return { x: event.clientX - (rect?.left ?? 0), y: event.clientY - (rect?.top ?? 0) }; };
  const worldPoint = (screen: Point): Point => ({ x: (screen.x - camera.x) / camera.zoom, y: (screen.y - camera.y) / camera.zoom });

  const commitPositions = async (next: Record<string, Point>) => {
    const changes = Object.entries(next).map(([id, point]) => ({ id, canvas: { position: [Math.round(point.x), Math.round(point.y)] as [number, number] } }));
    if (!changes.length) return;
    setHistory((stack) => [...stack, Object.fromEntries(Object.entries(positions).filter(([id]) => next[id]).map(([id, point]) => [id, point]))]);
    setRedo([]); setDraftPositions((current) => ({ ...current, ...next }));
    try { await store.mutateCanvas(changes); } catch { /* draft positions remain available for retry */ }
  };
  const pointerDownBoard = (event: React.PointerEvent) => {
    if (event.button === 1 || spaceDown.current) { setMode('pan'); setDrag({ start: { x: event.clientX, y: event.clientY }, origin: { x: camera.x, y: camera.y }, positions: {}, shift: false }); event.currentTarget.setPointerCapture(event.pointerId); return; }
    if (event.target !== event.currentTarget) return;
    if (!event.shiftKey) setSelected([]);
    const screen = screenPoint(event); setMode('marquee'); setMarquee({ a: screen, b: screen }); setDrag({ start: screen, origin: screen, positions: {}, shift: event.shiftKey }); event.currentTarget.setPointerCapture(event.pointerId);
  };
  const pointerMove = (event: React.PointerEvent) => {
    if (!drag) return;
    if (mode === 'pan') setCamera((current) => ({ ...current, x: drag.origin.x + event.clientX - drag.start.x, y: drag.origin.y + event.clientY - drag.start.y }));
    else if (mode === 'cards') {
      const point = screenPoint(event); const dx = (point.x - drag.start.x) / camera.zoom; const dy = (point.y - drag.start.y) / camera.zoom;
      const moved = Object.fromEntries(Object.entries(drag.positions).map(([id, original]) => [id, { x: Math.max(0, original.x + dx), y: Math.max(0, original.y + dy) }])); setDraftPositions((current) => ({ ...current, ...moved }));
    } else if (mode === 'marquee') setMarquee((current) => current ? { ...current, b: screenPoint(event) } : current);
  };
  const pointerUp = () => {
    if (mode === 'marquee' && marquee) {
      const a = worldPoint(marquee.a); const b = worldPoint(marquee.b); const left = Math.min(a.x,b.x); const top = Math.min(a.y,b.y); const right = Math.max(a.x,b.x); const bottom = Math.max(a.y,b.y);
      const inside = notes.filter((note) => { const p = positions[note.id]; return p.x < right && p.x + cardWidth > left && p.y < bottom && p.y + cardHeight > top; }).map((note) => note.id);
      setSelected((current) => marquee && drag?.shift ? [...new Set([...current, ...inside])] : inside);
    }
    if (mode === 'cards' && drag) {
      const changed = Object.fromEntries(Object.entries(draftPositions).filter(([id, p]) => drag.positions[id] && (p.x !== drag.positions[id].x || p.y !== drag.positions[id].y)));
      if (Object.keys(changed).length) void commitPositions(changed);
    }
    setMode('idle'); setDrag(null); setMarquee(null);
  };
  const beginCardDrag = (event: React.PointerEvent, id: string) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    const next = event.shiftKey ? selected.includes(id) ? selected.filter((item) => item !== id) : [...selected, id] : selected.includes(id) ? selected : [id];
    setSelected(next); setMode('cards');
    const origin = Object.fromEntries(next.map((item) => [item, positions[item]]));
    setDrag({ start: screenPoint(event), origin: { x: 0, y: 0 }, positions: origin, shift: event.shiftKey });
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };
  const align = (kind: 'left' | 'center' | 'right' | 'top' | 'middle' | 'bottom' | 'grid') => {
    if (selected.length < 2) return;
    const chosen = selected.map((id) => positions[id]); const minX = Math.min(...chosen.map((p) => p.x)); const maxX = Math.max(...chosen.map((p) => p.x)); const minY = Math.min(...chosen.map((p) => p.y)); const maxY = Math.max(...chosen.map((p) => p.y));
    const next: Record<string, Point> = {};
    selected.forEach((id, index) => {
      const p = positions[id];
      next[id] = kind === 'left' ? { ...p, x: minX } : kind === 'right' ? { ...p, x: maxX } : kind === 'center' ? { ...p, x: (minX + maxX) / 2 } : kind === 'top' ? { ...p, y: minY } : kind === 'bottom' ? { ...p, y: maxY } : kind === 'middle' ? { ...p, y: (minY + maxY) / 2 } : { x: minX + (index % Math.ceil(Math.sqrt(selected.length))) * 290, y: minY + Math.floor(index / Math.ceil(Math.sqrt(selected.length))) * 215 };
    });
    void commitPositions(next);
  };
  const undo = () => { const previous = history.at(-1); if (!previous) return; const current = Object.fromEntries(Object.keys(previous).map((id) => [id, positions[id]])); setHistory((items) => items.slice(0, -1)); setRedo((items) => [...items, current]); void store.mutateCanvas(Object.entries(previous).map(([id,p]) => ({ id, canvas: { position: [p.x,p.y] as [number,number] } }))).catch(() => undefined); };
  const redoPosition = () => { const next = redo.at(-1); if (!next) return; setRedo((items) => items.slice(0, -1)); setHistory((items) => [...items, Object.fromEntries(Object.keys(next).map((id) => [id, positions[id]]))]); void commitPositions(next); };
  const fit = () => { if (!host.current || !notes.length) { setCamera({ x: 0, y: 0, zoom: 1 }); return; } const points = notes.map((note) => positions[note.id]); const minX = Math.min(...points.map((p) => p.x)); const minY = Math.min(...points.map((p) => p.y)); const maxX = Math.max(...points.map((p) => p.x + cardWidth)); const maxY = Math.max(...points.map((p) => p.y + cardHeight)); const rect = host.current.getBoundingClientRect(); const zoom = Math.min(1.25, Math.max(.45, Math.min((rect.width - 120)/(maxX-minX), (rect.height - 130)/(maxY-minY)))); setCamera({ zoom, x: (rect.width-(maxX-minX)*zoom)/2-minX*zoom, y: (rect.height-(maxY-minY)*zoom)/2-minY*zoom }); };
  const keyHandler = (event: React.KeyboardEvent) => {
    if (event.key === ' ' && !isInteractiveTarget(event.target)) { spaceDown.current = true; event.preventDefault(); }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') { event.preventDefault(); setSelected(notes.map((note) => note.id)); }
    if (event.key === 'Escape') { if (selected.length) setSelected([]); else onBack(); }
    if (event.key === 'Enter' && selected.length === 1) { const note = noteById.get(selected[0]); if (note?.canvas.project) void store.bridge.openWindow('canvas', note.id); else onOpenNote(selected[0]); }
    if (event.altKey && ['a','h','d','w','v','s','t'].includes(event.key.toLowerCase())) { event.preventDefault(); const map: Record<string, Parameters<typeof align>[0]> = { a:'left',h:'center',d:'right',w:'top',v:'middle',s:'bottom',t:'grid' }; align(map[event.key.toLowerCase()]); }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') { event.preventDefault(); event.shiftKey ? redoPosition() : undo(); }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') { event.preventDefault(); redoPosition(); }
  };

  return <div className="canvas-screen" onKeyDown={keyHandler} onKeyUp={(event) => { if (event.key === ' ') spaceDown.current = false; }} onBlur={() => { spaceDown.current = false; }} tabIndex={0}>
    <header className="canvas-header"><div className="canvas-window-controls"><button aria-label="창 닫기" title="닫기" onClick={onBack}><Icon name="close"/></button><button aria-label="최소화" title="최소화" onClick={() => store.bridge.windowAction('minimize')}><Icon name="minimize"/></button><button aria-label="최대화" title="최대화" onClick={() => store.bridge.windowAction('maximize')}><Icon name="maximize"/></button></div><div className="canvas-title-area"><b>{title}</b></div><div className="canvas-header-actions"><button className="primary-button" onClick={() => void store.createNote({ parent: scope ?? null }).then((note) => note && onOpenNote(note.id)).catch(() => undefined)}><Icon name="plus"/> New</button></div></header>
    {selected.length > 1 && <div className="alignment-bar" role="toolbar" aria-label="선택 항목 정렬"><span>{selected.length}개 선택</span><button title="왼쪽 맞춤" onClick={() => align('left')}>⇤</button><button title="가운데 맞춤" onClick={() => align('center')}>↔</button><button title="오른쪽 맞춤" onClick={() => align('right')}>⇥</button><i/><button title="위쪽 맞춤" onClick={() => align('top')}>↑</button><button title="세로 가운데" onClick={() => align('middle')}>↕</button><button title="아래쪽 맞춤" onClick={() => align('bottom')}>↓</button><button title="격자로 정리" onClick={() => align('grid')}><Icon name="align" size={15}/></button></div>}
    <div ref={host} className={`canvas-board${mode === 'pan' ? ' panning' : ''}`} onPointerDown={pointerDownBoard} onPointerMove={pointerMove} onPointerUp={pointerUp} onPointerCancel={pointerUp} onWheel={(event) => { if (event.ctrlKey || event.metaKey) { event.preventDefault(); const point = screenPoint(event); setCamera((c) => { const zoom = Math.max(.3, Math.min(2.25, c.zoom * (event.deltaY < 0 ? 1.08 : .92))); return { zoom, x: point.x - (point.x-c.x)*zoom/c.zoom, y: point.y - (point.y-c.y)*zoom/c.zoom }; }); } }}>
      <div className="canvas-world" style={{ transform: `translate(${camera.x}px, ${camera.y}px) scale(${camera.zoom})` }}>
        <svg className="canvas-links" aria-hidden="true">{notes.flatMap((note) => note.canvas.next.map((target) => { const targetNote = noteById.get(target); if (!targetNote) return null; const a = positions[note.id]; const b = positions[target]; return <path key={`${note.id}-${target}`} d={`M ${a.x+cardWidth} ${a.y+cardHeight/2} C ${a.x+cardWidth+74} ${a.y+cardHeight/2}, ${b.x-74} ${b.y+cardHeight/2}, ${b.x} ${b.y+cardHeight/2}`} markerEnd="url(#arrow)"/>; }))}<defs><marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 L8 4 L0 8" fill="none" stroke="currentColor"/></marker></defs></svg>
        {notes.map((note) => { const position = positions[note.id]; const tasks = checklist(note.text); const checklistRows = note.text.split('\n').slice(1).map((line) => line.match(/^\s*[-*+]\s+\[([ xX])\]\s*(.*)$/)).filter((match): match is RegExpMatchArray => match !== null); const body = plainText(note.text.split('\n').slice(1).join(' ')).slice(0, 110) || '내용을 적어 보세요.'; return <article key={note.id} className={`canvas-note${selected.includes(note.id) ? ' selected' : ''}${note.canvas.done ? ' done' : ''}`} style={{ left: position.x, top: position.y, '--card-tone': colors[note.color] ?? colors.yellow } as React.CSSProperties} onPointerDown={(event) => beginCardDrag(event, note.id)} onDoubleClick={() => note.canvas.project ? void store.bridge.openWindow('canvas', note.id) : onOpenNote(note.id)} tabIndex={0} aria-label={`${titleOf(note)} 카드`}>
          <header><h3>{titleOf(note)}</h3>{note.canvas.done && <span className="done-badge"><Icon name="check" size={15}/></span>}{note.canvas.project && <button className="card-enter" title="프로젝트 들어가기" aria-label={`${titleOf(note)} 프로젝트 들어가기`} onPointerDown={(event) => event.stopPropagation()} onClick={() => void store.bridge.openWindow('canvas', note.id)}><Icon name="arrow" size={16}/></button>}</header>{checklistRows.length ? <ul className="canvas-checklist-preview">{checklistRows.slice(0, 3).map((row, index) => <li key={index}><span className={row[1].toLowerCase() === 'x' ? 'checked' : ''} aria-hidden="true">{row[1].toLowerCase() === 'x' ? <Icon name="check" size={12}/> : null}</span><small className={row[1].toLowerCase() === 'x' ? 'checked' : ''}>{plainText(row[2])}</small></li>)}</ul> : <p>{body}</p>}{tasks.total > 0 && <div className="canvas-card-tasks"><small>할 일 {tasks.done}/{tasks.total}</small></div>}{note.canvas.next.length > 0 && <footer><button aria-label={`${titleOf(note)} 다음 메모 연결`} title="연결점 끌어 메모 연결" onPointerDown={(event) => { event.stopPropagation(); setConnectFrom(note.id); setMode('connect'); }} onPointerUp={(event) => { event.stopPropagation(); if (connectFrom && connectFrom !== note.id) { const source = noteById.get(connectFrom); if (source && !source.canvas.next.includes(note.id)) void store.updateNote(source, undefined, { next: [...source.canvas.next, note.id] }).catch(() => undefined); } setConnectFrom(null); setMode('idle'); }}><Icon name="link" size={14}/></button><span>{note.canvas.next.length}</span></footer>}
        </article>; })}
      </div>
      {marquee && <div className="selection-box" style={{ left: Math.min(marquee.a.x,marquee.b.x), top: Math.min(marquee.a.y,marquee.b.y), width: Math.abs(marquee.a.x-marquee.b.x), height: Math.abs(marquee.a.y-marquee.b.y) }}/>}
      {!notes.length && <div className="canvas-empty"><div className="empty-mark"><Icon name="canvas"/></div><h2>이 캔버스는 비어 있어요</h2><p>새 메모를 만들거나 라이브러리에서 메모를 추가해 보세요.</p><button className="primary-button" onClick={() => void store.createNote({ parent: scope ?? null }).then((note) => note && onOpenNote(note.id)).catch(() => undefined)}><Icon name="plus"/> 새 메모</button></div>}
    </div>
    <footer className="canvas-dock"><button aria-label="실행 취소" title="실행 취소 Ctrl Z" onClick={undo} disabled={!history.length}><Icon name="undo"/></button><button aria-label="다시 실행" title="다시 실행 Ctrl Shift Z" onClick={redoPosition} disabled={!redo.length}><Icon name="redo"/></button><i/><button aria-label="축소" title="축소" onClick={() => setCamera((c) => ({ ...c, zoom: Math.max(.3, c.zoom - .1) }))}><Icon name="zoomOut"/></button><span>{Math.round(camera.zoom*100)}%</span><button aria-label="확대" title="확대" onClick={() => setCamera((c) => ({ ...c, zoom: Math.min(2.25, c.zoom + .1) }))}><Icon name="zoomIn"/></button><button className="canvas-fit-button" title="전체 보기" aria-label="전체 보기" onClick={fit}><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M7 3H4v3M13 3h3v3M4 13v3h3M16 13v3h-3"/></svg></button><button className="canvas-help-button" title="캔버스 도움말" aria-label="캔버스 도움말">?</button><i/><span className={`dock-save${store.error ? ' failed' : ''}`}><b/>{`메모 ${notes.length} · 연결 ${notes.reduce((count, note) => count + note.canvas.next.filter((id) => noteById.has(id)).length, 0)} ${store.error ? '저장 실패' : store.saving ? '저장 중' : '저장됨'}`}</span></footer>
  </div>;
}

function isInteractiveTarget(target: EventTarget): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('button, input, textarea, select, [contenteditable="true"]'));
}
