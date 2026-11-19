import { useRef } from 'react';
import type { KeyboardEvent, MutableRefObject } from 'react';
import type { Alignment } from '../../model/align';
import { alignmentForKey } from '../../model/alignment_actions';

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
  /** 마인드맵 가지 만들기. 처리했으면 true(Enter는 처리 못 하면 열기로 넘어간다). */
  addChild?: () => boolean; addSibling?: () => boolean;
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
    if (mod && event.key === 'Enter') { event.preventDefault(); actions.toggleDone?.(); return; }
    // Tab은 카드를 골랐고 입력 중이 아닐 때만 가로챈다. 그 밖에는 기본 포커스 이동.
    if (event.key === 'Tab' && !event.shiftKey && !event.altKey && actions.hasSelection() && !isInteractiveTarget(event.target) && actions.addChild?.()) { event.preventDefault(); return; }
    if (event.key === 'Enter' && !event.shiftKey && !event.altKey && !isInteractiveTarget(event.target) && actions.addSibling?.()) { event.preventDefault(); return; }
    if (event.key === 'Enter' && !isInteractiveTarget(event.target)) actions.openSelected(event.shiftKey);
    if (event.shiftKey && event.key === '1') { event.preventDefault(); actions.fit?.(false); }
    if (event.shiftKey && event.key === '2') { event.preventDefault(); actions.fit?.(true); }
    if (mod && event.key === '0') { event.preventDefault(); actions.resetZoom?.(); }
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key) && actions.hasSelection() && !isInteractiveTarget(event.target)) {
      event.preventDefault(); actions.nudge?.(event.key, event.shiftKey ? 80 : 16);
    }
    const alignment = alignmentForKey(event);
    if (alignment) { event.preventDefault(); actions.align(alignment); }
    if (event.altKey && event.key === 'ArrowUp') { event.preventDefault(); actions.goBack(); }
    if (event.altKey && event.key.toLowerCase() === 'g') { event.preventDefault(); event.shiftKey ? actions.liftSelected?.() : actions.groupSelected?.(); }
  };
  return { spaceDown, onKeyDown, onKeyUp: (event) => { if (event.key === ' ') spaceDown.current = false; }, onBlur: () => { spaceDown.current = false; } };
}
