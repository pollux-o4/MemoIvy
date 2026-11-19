import { SettingsButton } from '../../shared/SettingsButton';
import { useMemo, useRef, useState } from 'react';
import { ProgressProvider } from '../../shared/ProgressProvider';
import './canvas.css';
import type { Note } from '../../shared/engine_contract';
import { CreationMenu } from '../../shared/menu/CreationMenu';
import type { MemoStore } from '../../shared/useMemoivy';
import { WindowHeader } from '../../shared/WindowHeader';
import { AlignmentBar } from './parts/alignment/AlignmentBar';
import { CARD_HEIGHT, CARD_RADIUS, CARD_WIDTH, type CanvasPoint } from './model/align';

import type { CardActions } from './parts/CanvasCard';
import { CanvasWorld } from './parts/CanvasWorld';
import { CanvasDeleteDialog } from './parts/CanvasDeleteDialog';
import { CanvasDock } from './parts/CanvasDock';
import { CanvasEmpty } from './parts/CanvasEmpty';
import { SelectionBox } from './parts/SelectionBox';
import { CardMenu } from './parts/CardMenu';
import { CanvasInspector } from './parts/inspector/CanvasInspector';
import { CanvasNavigator } from './parts/CanvasNavigator';
import { readLinkStyle } from '../../shared/link_style';
import { canvasTitle } from './model/labels';
import { canMoveInto, MAX_PROJECT_DEPTH } from './model/scope_model';
import { useCardMenu } from './interaction/input/useCardMenu';
import { useBranchFocus } from './interaction/state/useBranchFocus';
import { useStableHandlers } from './interaction/state/useStableHandlers';
import { useSelectionActions } from './interaction/commands/useSelectionActions';
import { useCanvasClipboard } from './interaction/commands/useCanvasClipboard';
import { useCanvasCommands } from './interaction/commands/useCanvasCommands';
import { useCanvasKeyboard } from './interaction/input/useCanvasKeyboard';
import { useCanvasPointer } from './interaction/input/useCanvasPointer';
import { useCanvasScope } from './interaction/state/useCanvasScope';
import { useCanvasSelection } from './interaction/state/useCanvasSelection';
import { useCanvasCamera } from './interaction/camera/useCanvasCamera';
import { useCanvasLayout } from './interaction/state/useCanvasLayout';
import { useFanQuiet } from './interaction/state/useFanQuiet';

// 카드 치수의 원본은 model/align. CSS(카드·부채)는 이 변수만 읽는다.
const CARD_SIZE_VARS = { '--card-width': `${CARD_WIDTH}px`, '--card-height': `${CARD_HEIGHT}px`, '--card-radius': `${CARD_RADIUS}px` } as React.CSSProperties;

/** 캔버스 화면 조립: 범위·선택·카메라·명령·포인터 훅을 묶고 화면 부품에 전달한다. */
export function Canvas({ store, scope, onOpenNote, onBack, onHelp, onOpenSettings }: { store: MemoStore; scope?: string; onOpenNote: (id: string) => void; onBack: () => void; onHelp: () => void; onOpenSettings: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const [deleteTarget, setDeleteTarget] = useState<string[]>([]);
  const openLinkPicker = useRef<(() => void) | null>(null);
  const [renameRequest, setRenameRequest] = useState(0);
  const [dragPositions, setDragPositions] = useState<Record<string, CanvasPoint>>({});
  const [pendingPositions, setPendingPositions] = useState<Record<string, CanvasPoint>>({});
  const draftPositions = useMemo(() => ({ ...pendingPositions, ...dragPositions }), [pendingPositions, dragPositions]);
  const selection = useCanvasSelection();
  const { selected, setSelected, mode } = selection;
  const scopeState = useCanvasScope(store.notes, scope, setSelected, onBack);
  const { activeScope, scopedProject } = scopeState;
  const notes = useMemo(() => store.notes.filter((note) => (note.canvas.parent ?? null) === activeScope), [store.notes, activeScope]);
  const { noteById, positions, cardRects, selectedRects } = useCanvasLayout(notes, draftPositions, selected);
  const { motion, camera, triggerLink, reveal, fitView, resetZoom } = useCanvasCamera({ host, notes, noteById, scope: activeScope, cardRects, selected, selectedRects, idle: mode === 'idle' });
  const commands = useCanvasCommands({ store, notes, positions, selected, activeScope, setDraftPositions: setPendingPositions, triggerLink });
  const clipboard = useCanvasClipboard({ store, notes, positions, selected, activeScope, onPasted: (ids) => selectAndReveal(ids) });
  const movable = (target: string, ids: string[]) => canMoveInto(target, ids, store.notes);
  const selectAndReveal = (ids: string[]) => { setSelected(ids); reveal(); };
  const cardMenu = useCardMenu(host, setSelected);

  const { focus, outlineRows, indent, outdent, move, shift } = useBranchFocus({ allNotes: store.notes, scope: activeScope, noteById, selected, commit: (changes) => commands.commit(changes) });
  const linkStyle = readLinkStyle(store.snapshot?.preferences);
  const title = scopedProject ? canvasTitle(scopedProject) : '메모 캔버스';
  const selectedNotes = notes.filter((note) => selected.includes(note.id));
  const selectedNote = selectedNotes.length === 1 ? selectedNotes[0] : undefined;
  const openCard = (note: Note, editProject = false) => note.canvas.project && !editProject ? scopeState.navigate(note.id) : onOpenNote(note.id);
  const actions = useSelectionActions({ store, selected, selectedNote, openCard, clipboard, toggleDone: commands.toggleDone, requestDelete: setDeleteTarget, requestRename: () => setRenameRequest((count) => count + 1) });
  const createItem = (project: boolean) => { void store.createNote({ project, parent: activeScope }).then((note) => { if (note) { selectAndReveal([note.id]); onOpenNote(note.id); } }).catch(() => undefined); };
  // 새 가지는 고르고 바로 이름 칸에 포커스한다(생각하고 Tab, 다시 생각).
  const branch = (kind: 'child' | 'sibling', from?: { seed: string; at?: CanvasPoint }) => { if (store.saving) return; void commands.addBranch(kind, from).then((note) => { if (note) { selectAndReveal([note.id]); setRenameRequest((count) => count + 1); } }).catch(() => undefined); };
  const groupSelected = async () => {
    if (!selected.length || store.saving || mode !== 'idle' || scopeState.depth >= MAX_PROJECT_DEPTH) return;
    try { const project = await commands.group(); if (project) { setSelected([project.id]); onOpenNote(project.id); } } catch { /* 저장 오류는 상태 막대가 알린다 */ }
  };
  const liftSelected = () => { if (scopedProject && selected.length) commands.moveInto(selected, scopedProject.canvas.parent ?? null); };
  const fan = useFanQuiet();
  const pointer = useCanvasPointer({
    host, camera, setCamera: motion.setCamera, setAutopan: motion.setAutopan, startCoast: motion.startCoast, isPanHeld: () => keyboard.spaceDown.current,
    ...selection, notes, allNotes: store.notes, positions, draftPositions: dragPositions, setDraftPositions: setDragPositions, canMoveInto: movable,
    onCardsMoved: fan.hush, onMovePositions: commands.commitPositions, onMoveInto: commands.moveInto,
    onConnectEmpty: (from, at) => branch('child', { seed: from, at }),
    onConnect: (from, to) => { const source = noteById.get(from); if (source && !source.canvas.next.includes(to)) commands.link(source, to); },
  });
  const keyboard = useCanvasKeyboard({
    selectAll: () => setSelected(notes.map((note) => note.id)), clearSelection: () => setSelected([]), goBack: scopeState.leave, hasSelection: () => selected.length > 0,
    openSelected: (shift) => { if (selectedNote) openCard(selectedNote, shift); },
    align: commands.align, undo: commands.history.undo, redo: commands.history.redo,
    removeSelected: actions.remove, toggleDone: actions.toggleDone,
    connectSelected: () => openLinkPicker.current?.(),
    fit: fitView, resetZoom, groupSelected: () => void groupSelected(), liftSelected,
    nudge: commands.nudge, cancelInteraction: pointer.cancel,
    copy: actions.copy, cut: actions.cut, paste: actions.paste, duplicate: actions.duplicate, rename: actions.rename,
    openMenu: () => cardMenu.openForSelection(selected),
    addChild: () => { if (selected.length !== 1 || store.saving) return false; branch('child'); return true; },
    addSibling: () => { if (!commands.canAddSibling() || store.saving) return false; branch('sibling'); return true; },
  });

  // 카드·인스펙터 동작은 최신 상태를 읽되 참조는 고정한다(끌기 중 카드 200장·인스펙터가 다시 그려지지 않게).
  const cardActions: CardActions = useStableHandlers({
    pointerDown: pointer.beginCardDrag,
    pointerLeave: fan.wake,
    open: (note: Note) => openCard(note),
    connectStart: pointer.beginConnection,
    addBranch: (id: string) => branch('child', { seed: id }),
    toggleTask: actions.toggleTask,
    toggleDone: (id: string) => commands.toggleDone([id]),
    contextMenu: (event: React.MouseEvent, id: string) => { event.preventDefault(); cardMenu.openAt(selected.includes(id) ? selected : [id], { x: event.clientX, y: event.clientY }); },
  });
  const inspectorActions = useStableHandlers({
    external: (url: string) => void store.bridge.openExternal(url),
    saveText: actions.saveText,
    color: (note: Note, color: string) => actions.recolor([note.id], color),
    close: () => setSelected([]),
    addBranch: () => { if (!store.saving) branch('child'); },
    open: (note: Note) => openCard(note),
    openAsNote: (note: Note) => openCard(note, true),
    lift: liftSelected,
    moveInto: (parent: string) => commands.moveInto(selected, parent),
    toggleDone: () => { if (selectedNote) commands.toggleDone([selectedNote.id]); },
    signal: () => { if (selectedNote) for (const id of selectedNote.canvas.next) triggerLink(`${selectedNote.id}:${id}`, false); },
    remove: () => { if (selectedNote) setDeleteTarget([selectedNote.id]); },
    select: (id: string) => selectAndReveal([id]),
    toggleLink: commands.link,
    indent: (id: string) => indent(id),
    outdent: (id: string) => outdent(id),
    move: (id: string, target: string, place: 'before' | 'after') => move(id, target, place),
    shift: (id: string, step: -1 | 1) => shift(id, step),
  });
  const moveTargets = useMemo(() => selectedNote ? store.notes.filter((note) => note.canvas.project && note.canvas.parent !== activeScope && canMoveInto(note.id, [selectedNote.id], store.notes)) : [], [store.notes, activeScope, selectedNote]);
  const inspectorOutline = useMemo(() => ({ rows: outlineRows, onIndent: inspectorActions.indent, onOutdent: inspectorActions.outdent, onMove: inspectorActions.move, onShift: inspectorActions.shift }), [outlineRows, inspectorActions]);

  const confirmDelete = async () => {
    for (const id of deleteTarget) {
      try { await store.removeNote(id); } catch { break; }
    }
    setSelected([]);
    setDeleteTarget([]);
  };
  const preview = pointer.connectPreview; const connectFrom = selection.connectFrom ? positions[selection.connectFrom] : undefined;
  const connectView = useMemo(() => preview && connectFrom ? { from: connectFrom, to: preview.to, target: preview.target ? positions[preview.target] : undefined, targetId: preview.target } : null, [preview, connectFrom, positions]);
  const hasProjects = store.notes.some((note) => note.canvas.project);
  // 가지 초점: 흐림은 world의 has-focus가 정하고 카드·선은 가지 안인지만 받는다. 초점이 바뀌어도 가지 밖 카드 200장은 다시 그리지 않는다.
  // 단축키는 화면 루트가 포커스를 받아 처리한다. application 역할은 보드에만 둬 제목 줄·도구 막대가 랜드마크로 남게 한다.
  // biome-ignore lint/a11y/noNoninteractiveTabindex: 캔버스 단축키를 받으려면 화면 자체가 포커스를 받아야 한다.
  // biome-ignore lint/a11y/noStaticElementInteractions: 키 입력은 안쪽 컨트롤에서 올라온 것을 모아 처리한다.
  return <ProgressProvider notes={store.notes}><div className="canvas-screen" style={CARD_SIZE_VARS} onKeyDown={keyboard.onKeyDown} onKeyUp={keyboard.onKeyUp} onBlur={keyboard.onBlur} tabIndex={0}>
    <WindowHeader className="window-header canvas-header" title={title} bridge={store.bridge} onClose={onBack} actions={<>
      <SettingsButton onClick={onOpenSettings}/>
      <CreationMenu onCreate={createItem} projectAllowed={scopeState.depth < MAX_PROJECT_DEPTH}/>
    </>}/>
    {selected.length > 1 && <AlignmentBar count={selected.length} onAlign={commands.align} onDelete={() => setDeleteTarget(selected)} onGroup={() => void groupSelected()} onLift={scopedProject ? liftSelected : undefined}/>}
    {(scopeState.path.length > 0 || hasProjects) && <CanvasNavigator path={scopeState.path} shortcuts={scopeState.shortcuts} scopedProject={scopedProject} onNavigate={scopeState.navigate} onTogglePin={(project) => void store.updateNote(project, undefined, { pinned: !project.canvas.pinned })}/>}
    <div ref={host} className={`canvas-board${mode === 'pan' ? ' panning' : ''}${pointer.connectPicking ? ' picking' : ''}`} role="application" aria-label="캔버스"
      {...pointer.board}
>
      <CanvasWorld camera={camera} notes={notes} allNotes={store.notes} scope={activeScope} positions={positions} selected={selected} focus={focus}
        quiet={fan.quiet} dropTarget={pointer.dropTarget} connect={connectView} guides={pointer.guides} linkStyle={linkStyle} sampleForLink={motion.sampleForLink} actions={cardActions}/>
      {cardMenu.menu && <CardMenu at={cardMenu.menu.at} notes={selectedNotes} actions={actions} onClose={cardMenu.close}/>}
      {pointer.connectPicking && <p className="canvas-pick-hint" role="status">이을 메모를 누르세요 · 빈 곳은 새 가지 · Esc 취소</p>}
      {selection.marquee && <SelectionBox a={selection.marquee.a} b={selection.marquee.b}/>}
      {!notes.length && <CanvasEmpty onCreate={() => void store.createNote({ parent: activeScope }).then((note) => note && selectAndReveal([note.id])).catch(() => undefined)}/>}
    </div>
    {selectedNote && <CanvasInspector note={selectedNote} notes={notes} renameRequest={renameRequest} revision={store.snapshot?.revision} projects={moveTargets}
      onExternal={inspectorActions.external} onSaveText={inspectorActions.saveText} onToggleTask={cardActions.toggleTask} onColor={inspectorActions.color}
      onClose={inspectorActions.close} onOpen={inspectorActions.open} onOpenAsNote={inspectorActions.openAsNote}
      onLift={scopedProject ? inspectorActions.lift : undefined} onMoveInto={inspectorActions.moveInto}
      onToggleDone={inspectorActions.toggleDone} onSignal={inspectorActions.signal} onDelete={inspectorActions.remove}
      onSelect={inspectorActions.select} openLinkPicker={openLinkPicker} onToggleLink={inspectorActions.toggleLink} onAddBranch={inspectorActions.addBranch} outline={inspectorOutline}/>}
    {deleteTarget.length > 0 && <CanvasDeleteDialog ids={deleteTarget} notes={store.notes} onCancel={() => setDeleteTarget([])} onAccept={() => void confirmDelete()}/>}
    <CanvasDock count={notes.length}
      links={notes.reduce((count, note) => count + note.canvas.next.filter((id) => noteById.has(id)).length, 0)}
      zoom={camera.zoom} saving={store.saving} error={store.error}
      canUndo={commands.history.canUndo} canRedo={commands.history.canRedo} onUndo={commands.history.undo} onRedo={commands.history.redo}
      onZoom={motion.zoomByStep} onFit={() => fitView()} onReset={resetZoom}
      onHelp={onHelp}/>
  </div></ProgressProvider>;
}
