import { useRef } from 'react';
import type { KeyboardEvent, MutableRefObject } from 'react';
import type { Alignment } from '../model/align';

const alignmentKeys: Record<string, Alignment> = { a: 'left', h: 'center', d: 'right', w: 'top', v: 'middle', s: 'bottom', t: 'grid', f: 'flow' };

function isTypingTarget(target: EventTarget): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('input, textarea, select, [contenteditable="true"]'));
}

function isInteractiveTarget(target: EventTarget): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('button, input, textarea, select, [contenteditable="true"]'));
}

export function useCanvasKeyboard(actions: {
  selectAll: () => void; clearSelection: () => void; goBack: () => void; openSelected: (shift: boolean) => void;
  hasSelection: () => boolean; align: (kind: Alignment) => void; undo: () => void; redo: () => void;
  removeSelected?: () => void; toggleDone?: () => void; connectSelected?: () => void; fit?: (selection: boolean) => void; resetZoom?: () => void;
  nudge?: (key: string, step: number) => void;
  cancelInteraction?: () => boolean;
  groupSelected?: () => void; liftSelected?: () => void;
  copy?: () => void; cut?: () => void; paste?: () => void; duplicate?: () => void;
  rename?: () => void; openMenu?: () => void;
}): { spaceDown: MutableRefObject<boolean>; onKeyDown: (event: KeyboardEvent) => void; onKeyUp: (event: KeyboardEvent) => void; onBlur: () => void } {
  const spaceDown = useRef(false);
  /** Ctrl(⌘)+글자 단축키 표. 새 단축키는 여기에 한 줄 추가한다. 입력 칸에서는 기본 동작(글 복사 등)에 맡긴다. */
  const modCommands: Record<string, (() => void) | undefined> = {
    a: actions.selectAll,
    l: actions.connectSelected,
    z: actions.undo, 'shift+z': actions.redo, y: actions.redo,
    c: actions.copy,
    x: actions.cut, v: actions.paste, d: actions.duplicate,
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.target instanceof HTMLElement && event.target.closest('[role="alertdialog"], [role="menu"]')) return;
    if (event.key === ' ' && !isInteractiveTarget(event.target)) { spaceDown.current = true; event.preventDefault(); }
    if (event.target instanceof HTMLElement && event.target.closest('input, textarea, select, [contenteditable="true"]') && ['Delete', 'Backspace', 'Enter', 'ArrowDown', 'ArrowUp'].includes(event.key)) return;
    const mod = event.ctrlKey || event.metaKey;
    const command = mod && !event.altKey && !isTypingTarget(event.target) ? modCommands[`${event.shiftKey ? 'shift+' : ''}${event.key.toLowerCase()}`] : undefined;
    if (command) { event.preventDefault(); command(); return; }
    if (!isTypingTarget(event.target) && actions.hasSelection()) {
      if (event.key === 'F2') { event.preventDefault(); actions.rename?.(); return; }
      if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) { event.preventDefault(); actions.openMenu?.(); return; }
    }
    if (event.key === 'Escape') { if (actions.cancelInteraction?.()) return; if (actions.hasSelection()) actions.clearSelection(); else actions.goBack(); }
    if ((event.key === 'Delete' || event.key === 'Backspace') && actions.hasSelection()) { event.preventDefault(); actions.removeSelected?.(); }
    if (mod && event.key === 'Enter') { event.preventDefault(); actions.toggleDone?.(); }
    if (event.key === 'Enter' && !isInteractiveTarget(event.target)) actions.openSelected(event.shiftKey);
    if (event.altKey && event.shiftKey && !event.ctrlKey && ['h', 'v'].includes(event.key.toLowerCase())) { event.preventDefault(); actions.align(event.key.toLowerCase() === 'h' ? 'distribute-x' : 'distribute-y'); }
    if (event.shiftKey && event.key === '1') { event.preventDefault(); actions.fit?.(false); }
    if (event.shiftKey && event.key === '2') { event.preventDefault(); actions.fit?.(true); }
    if (mod && event.key === '0') { event.preventDefault(); actions.resetZoom?.(); }
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key) && actions.hasSelection() && !isInteractiveTarget(event.target)) {
      event.preventDefault(); actions.nudge?.(event.key, event.shiftKey ? 80 : 16);
    }
    if (event.altKey && !event.ctrlKey && !event.shiftKey && alignmentKeys[event.key.toLowerCase()]) { event.preventDefault(); actions.align(alignmentKeys[event.key.toLowerCase()]); }
    if (event.altKey && event.key === 'ArrowUp') { event.preventDefault(); actions.goBack(); }
    if (event.altKey && event.key.toLowerCase() === 'g') { event.preventDefault(); event.shiftKey ? actions.liftSelected?.() : actions.groupSelected?.(); }
  };
  return { spaceDown, onKeyDown, onKeyUp: (event) => { if (event.key === ' ') spaceDown.current = false; }, onBlur: () => { spaceDown.current = false; } };
}
