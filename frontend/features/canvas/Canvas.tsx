import { useEffect, useMemo, useRef, useState } from 'react';
import './canvas.css';
import { ConfirmDialog } from '../../shared/ConfirmDialog';
import type { Note } from '../../shared/engine_contract';
import { Icon } from '../../shared/Icon';
import { CreationMenu } from '../../shared/menu/CreationMenu';
import type { MemoStore } from '../../shared/useMemoivy';
import { WindowHeader } from '../../shared/WindowHeader';
import { AlignmentBar } from './parts/alignment/AlignmentBar';
import { canvasSlot, CARD_HEIGHT, CARD_WIDTH, cardRect, type CanvasPoint } from './model/align';

import { CanvasCard } from './parts/CanvasCard';
import { CanvasDock } from './parts/CanvasDock';
import { CardMenu } from './parts/CardMenu';
import { CanvasInspector } from './parts/inspector/CanvasInspector';
import { CanvasLinks } from './parts/CanvasLinks';
import { CanvasNavigator } from './parts/CanvasNavigator';
import { SnapGuides } from './parts/SnapGuides';
import { gridStyle } from './model/grid';
import { canvasTitle } from './model/labels';
import { bounds, fitting } from './model/motion';
import { canMoveInto } from './model/scope_model';
import { useCardMenu } from './interaction/useCardMenu';
import { useSelectionActions } from './interaction/useSelectionActions';
import { useCanvasClipboard } from './interaction/useCanvasClipboard';
import { useCanvasCommands } from './interaction/useCanvasCommands';
import { useCanvasKeyboard } from './interaction/useCanvasKeyboard';
import { useCanvasMotion } from './interaction/useCanvasMotion';
import { useCanvasPointer } from './interaction/useCanvasPointer';
import { useCanvasScope } from './interaction/useCanvasScope';
import { useCanvasSelection } from './interaction/useCanvasSelection';
import { useCanvasWheel } from './interaction/useCanvasWheel';
import { useRevealSelection } from './interaction/useRevealSelection';
import { useFanQuiet } from './interaction/useFanQuiet';

// 카드 치수의 원본은 model/align. CSS(카드·부채)는 이 변수만 읽는다.
const CARD_SIZE_VARS = { '--card-width': `${CARD_WIDTH}px`, '--card-height': `${CARD_HEIGHT}px` } as React.CSSProperties;

const MAX_DEPTH = 3;

/** 캔버스 화면 조립: 범위·선택·카메라·명령·포인터 훅을 묶고 화면 부품에 전달한다. */
export function Canvas({ store, scope, onOpenNote, onBack }: { store: MemoStore; scope?: string; onOpenNote: (id: string) => void; onBack: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const [deleteTarget, setDeleteTarget] = useState<string[]>([]);
  const [focusConnectList, setFocusConnectList] = useState(false);
  const [renameRequest, setRenameRequest] = useState(0);
  const [draftPositions, setDraftPositions] = useState<Record<string, CanvasPoint>>({});
  const selection = useCanvasSelection();
  const { selected, setSelected, mode } = selection;
  const scopeState = useCanvasScope(store.notes, scope, setSelected, onBack);
  const { activeScope, scopedProject } = scopeState;
  const notes = useMemo(() => store.notes.filter((note) => (note.canvas.parent ?? null) === activeScope), [store.notes, activeScope]);
  const noteById = useMemo(() => new Map(notes.map((note) => [note.id, note])), [notes]);
  const positions = useMemo(() => Object.fromEntries(notes.map((note, index) => {
    const saved = note.canvas.position ? { x: note.canvas.position[0], y: note.canvas.position[1] } : canvasSlot(index);
    return [note.id, draftPositions[note.id] ?? saved];
  })), [notes, draftPositions]);
  const cardRects = useMemo(() => notes.map((note) => cardRect(positions[note.id])), [notes, positions]);
  const selectedRects = useMemo(() => selected.flatMap((id) => positions[id] ? [cardRect(positions[id])] : []), [selected, positions]);
  const motion = useCanvasMotion(host, cardRects, selectedRects);
  const { camera, animateTo, triggerLink } = motion;
  useCanvasWheel(host, motion);
  const reveal = useRevealSelection(host, selectedRects.length === 1 ? selectedRects[0] : undefined, camera, animateTo);
  const commands = useCanvasCommands({ store, notes, positions, selected, activeScope, setDraftPositions, triggerLink });
  const clipboard = useCanvasClipboard({ store, notes, positions, selected, activeScope, onPasted: (ids) => selectAndReveal(ids) });
  const movable = (target: string, ids: string[]) => canMoveInto(target, ids, store.notes);
  const selectAndReveal = (ids: string[]) => { setSelected(ids); reveal(); };
  const cardMenu = useCardMenu(host, setSelected);

  const title = scopedProject ? canvasTitle(scopedProject) : '메모 캔버스';
  const selectedNotes = notes.filter((note) => selected.includes(note.id));
  const selectedNote = selectedNotes.length === 1 ? selectedNotes[0] : undefined;
  const openCard = (note: Note, editProject = false) => note.canvas.project && !editProject ? scopeState.navigate(note.id) : onOpenNote(note.id);
  const actions = useSelectionActions({ store, selected, selectedNote, openCard, clipboard, toggleDone: commands.toggleDone, requestDelete: setDeleteTarget, requestRename: () => setRenameRequest((count) => count + 1) });
  const createItem = (project: boolean) => { void store.createNote({ project, parent: activeScope }).then((note) => { if (note) { selectAndReveal([note.id]); onOpenNote(note.id); } }).catch(() => undefined); };
  const groupSelected = async () => {
    if (!selected.length || store.saving || mode !== 'idle' || scopeState.depth >= MAX_DEPTH) return;
    try { const project = await commands.group(); if (project) { setSelected([project.id]); onOpenNote(project.id); } } catch { /* 저장 오류는 상태 막대가 알린다 */ }
  };
  const liftSelected = () => { if (scopedProject && selected.length) commands.moveInto(selected, scopedProject.canvas.parent ?? null); };
  const fit = (onlySelection = false) => {
    const board = host.current?.getBoundingClientRect();
    const target = bounds(onlySelection ? selectedRects : cardRects);
    if (board && target) animateTo(fitting(target, [board.width, board.height]));
  };
  const resetZoom = () => {
    if (!host.current) return;
    const { width, height } = host.current.getBoundingClientRect();
    animateTo({ x: camera.x + width / camera.zoom / 2 - width / 2, y: camera.y + height / camera.zoom / 2 - height / 2, zoom: 1 });
  };
  const fan = useFanQuiet();
  const pointer = useCanvasPointer({
    host, camera, setCamera: motion.setCamera, setAutopan: motion.setAutopan, startCoast: motion.startCoast, isPanHeld: () => keyboard.spaceDown.current,
    ...selection, notes, allNotes: store.notes, positions, draftPositions, setDraftPositions, canMoveInto: movable,
    onCardsMoved: fan.hush, onMovePositions: commands.commitPositions, onMoveInto: commands.moveInto,
    onConnect: (from, to) => { const source = noteById.get(from); if (source && !source.canvas.next.includes(to)) commands.link(source, to); },
  });
  const keyboard = useCanvasKeyboard({
    selectAll: () => setSelected(notes.map((note) => note.id)), clearSelection: () => setSelected([]), goBack: scopeState.leave, hasSelection: () => selected.length > 0,
    openSelected: (shift) => { if (selectedNote) openCard(selectedNote, shift); },
    align: commands.align, undo: commands.history.undo, redo: commands.history.redo,
    removeSelected: actions.remove, toggleDone: actions.toggleDone,
    connectSelected: () => { if (selected.length === 1) setFocusConnectList(true); },
    fit, resetZoom, groupSelected: () => void groupSelected(), liftSelected,
    nudge: commands.nudge, cancelInteraction: pointer.cancel,
    copy: actions.copy, cut: actions.cut, paste: actions.paste, duplicate: actions.duplicate, rename: actions.rename,
    openMenu: () => cardMenu.openForSelection(selected),
  });

  const previousScope = useRef(activeScope);
  useEffect(() => {
    if (previousScope.current === activeScope) return;
    previousScope.current = activeScope;
    const frame = requestAnimationFrame(() => fit());
    return () => cancelAnimationFrame(frame);
  }, [activeScope]);
  const seededLinks = useRef(false);
  useEffect(() => {
    if (seededLinks.current || !notes.length) return;
    seededLinks.current = true;
    notes.forEach((note) => note.canvas.next.forEach((target) => { if (noteById.has(target)) triggerLink(`${note.id}:${target}`, false); }));
  }, [notes, noteById, triggerLink]);
  useEffect(() => {
    if (!focusConnectList || !selectedNote) return;
    const frame = requestAnimationFrame(() => {
      const select = host.current?.parentElement?.querySelector<HTMLSelectElement>('.inspector-add-link select');
      select?.focus();
      try { select?.showPicker(); } catch { /* focus remains the keyboard fallback */ }
      setFocusConnectList(false);
    });
    return () => cancelAnimationFrame(frame);
  }, [focusConnectList, selectedNote]);
  const confirmDelete = async () => {
    for (const id of deleteTarget) {
      try { await store.removeNote(id); } catch { break; }
    }
    setSelected([]);
    setDeleteTarget([]);
  };
  const deleteHasLinks = deleteTarget.length > 1 || store.notes.some((note) =>
    deleteTarget.includes(note.id) && note.canvas.next.length > 0 || note.canvas.next.some((target) => deleteTarget.includes(target)),
  );
  const deleteSubject = deleteTarget.length === 1 ? store.notes.find((note) => note.id === deleteTarget[0]) : undefined;
  const hasProjects = store.notes.some((note) => note.canvas.project);
  return <div className="canvas-screen" style={CARD_SIZE_VARS} onKeyDown={keyboard.onKeyDown} onKeyUp={keyboard.onKeyUp} onBlur={keyboard.onBlur} tabIndex={0}>
    <WindowHeader className="canvas-header" title={title} bridge={store.bridge} onClose={onBack} actions={<CreationMenu onCreate={createItem} projectAllowed={scopeState.depth < MAX_DEPTH}/>}/>
    {selected.length > 1 && <AlignmentBar count={selected.length} onAlign={commands.align} onDelete={() => setDeleteTarget(selected)} onGroup={() => void groupSelected()} onLift={scopedProject ? liftSelected : undefined}/>}
    {(scopeState.path.length > 0 || hasProjects) && <CanvasNavigator path={scopeState.path} shortcuts={scopeState.shortcuts} scopedProject={scopedProject} onNavigate={scopeState.navigate} onTogglePin={(project) => void store.updateNote(project, undefined, { pinned: !project.canvas.pinned })}/>}
    <div ref={host} className={`canvas-board${mode === 'pan' ? ' panning' : ''}`} style={gridStyle(camera) as React.CSSProperties}
      {...pointer.board}
>
      <div className="canvas-world" style={{ transform: `translate(${-camera.x * camera.zoom}px, ${-camera.y * camera.zoom}px) scale(${camera.zoom})` }}>
        <CanvasLinks notes={notes} allNotes={store.notes} scope={activeScope} positions={positions} zoom={camera.zoom} selected={selected} sampleForLink={motion.sampleForLink}/>
        {notes.map((note) => <CanvasCard key={note.id} note={note} allNotes={store.notes} dropTarget={pointer.dropTarget === note.id} onToggleDone={() => commands.toggleDone([note.id])} position={positions[note.id]} selected={selected.includes(note.id)} onPointerDown={pointer.beginCardDrag} fanQuiet={fan.quiet.includes(note.id)} onPointerLeave={fan.wake}
          onOpen={(item) => openCard(item)} onConnectStart={pointer.beginConnection}
          onContextMenu={(event, id) => { event.preventDefault(); cardMenu.openAt(selected.includes(id) ? selected : [id], { x: event.clientX, y: event.clientY }); }}/>)}
        <SnapGuides guides={pointer.guides} zoom={camera.zoom}/>
      </div>
      {cardMenu.menu && <CardMenu at={cardMenu.menu.at} notes={selectedNotes} actions={actions} onClose={cardMenu.close}/>}
      {selection.marquee && <div className="selection-box" style={{ left: Math.min(selection.marquee.a.x, selection.marquee.b.x), top: Math.min(selection.marquee.a.y, selection.marquee.b.y), width: Math.abs(selection.marquee.a.x - selection.marquee.b.x), height: Math.abs(selection.marquee.a.y - selection.marquee.b.y) }}/>}
      {!notes.length && <div className="canvas-empty">
        <div className="empty-mark"><Icon name="canvas"/></div>
        <h2>이 캔버스는 비어 있어요</h2>
        <p>새 메모를 만들거나 라이브러리에서 메모를 추가해 보세요.</p>
        <button className="primary-button" onClick={() => void store.createNote({ parent: activeScope }).then((note) => note && selectAndReveal([note.id])).catch(() => undefined)}>
          <Icon name="plus"/> 새 메모
        </button>
      </div>}
    </div>
    {selectedNote && <CanvasInspector note={selectedNote} notes={notes} allNotes={store.notes} renameRequest={renameRequest}
      onSaveText={actions.saveText} onColor={(note, color) => actions.recolor([note.id], color)}
      projects={store.notes.filter((note) => note.canvas.project && note.canvas.parent !== activeScope && movable(note.id, [selectedNote.id]))}
      onClose={() => setSelected([])} onOpen={(note) => openCard(note)}
      onLift={scopedProject ? liftSelected : undefined} onMoveInto={(parent) => commands.moveInto(selected, parent)}
      onToggleDone={() => commands.toggleDone([selectedNote.id])}
      onSignal={() => selectedNote.canvas.next.forEach((id) => triggerLink(`${selectedNote.id}:${id}`, false))}
      onDelete={() => setDeleteTarget([selectedNote.id])} onSelect={(id) => selectAndReveal([id])}
      onToggleLink={commands.link}/>}
    {deleteTarget.length > 0 && <ConfirmDialog
      question={deleteTarget.length === 1 ? '이 메모를 삭제할까요?' : `메모 ${deleteTarget.length}개를 삭제할까요?`}
      subject={deleteSubject ? canvasTitle(deleteSubject) : undefined}
      detail={deleteHasLinks
        ? '삭제한 메모는 복구할 수 없습니다. 내용과 연결이 모두 사라져요.'
        : '삭제한 메모는 복구할 수 없습니다.'}
      acceptLabel="삭제" onCancel={() => setDeleteTarget([])} onAccept={() => void confirmDelete()}/>}
    <CanvasDock count={notes.length}
      links={notes.reduce((count, note) => count + note.canvas.next.filter((id) => noteById.has(id)).length, 0)}
      zoom={camera.zoom} saving={store.saving} error={store.error}
      canUndo={commands.history.canUndo} canRedo={commands.history.canRedo} onUndo={commands.history.undo} onRedo={commands.history.redo}
      onZoom={motion.zoomByStep} onFit={fit} onReset={resetZoom}
      onHelp={() => void store.bridge.openWindow('help')}/>
  </div>;
}
