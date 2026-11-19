import type { EditorView } from '@codemirror/view';
import { imageDropIndent, moveImageLine } from './commands';

export function installImageDropHandlers(view: EditorView) {
  let source: { line: number; doc: EditorView['state']['doc']; element: HTMLElement } | undefined;
  let marker: HTMLElement | undefined;
  const clearMarker = () => {
    marker?.removeAttribute('data-image-drop');
    marker?.style.removeProperty('--image-drop-inset');
    marker = undefined;
  };
  const clearDrag = () => {
    clearMarker();
    source?.element.removeAttribute('data-image-dragging');
    source = undefined;
  };
  const dragStart = (event: DragEvent) => {
    const element = (event.target as HTMLElement | null)?.closest<HTMLElement>('.cm-asset-preview');
    if (!element?.dataset.assetHash || !event.dataTransfer) return;
    const doc = view.state.doc;
    const line = doc.lineAt(view.posAtDOM(element));
    source = { line: line.from, doc, element };
    element.dataset.imageDragging = '';
    event.dataTransfer.effectAllowed = 'copyMove';
    event.dataTransfer.setData('text/plain', line.text);
  };
  const targetAt = (event: DragEvent) => {
    const element = (event.target as HTMLElement | null)?.closest<HTMLElement>('.cm-line');
    const position = element ? view.posAtDOM(element) : view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (position == null) return;
    const line = view.state.doc.lineAt(position);
    const box = element?.getBoundingClientRect() ?? view.lineBlockAt(line.from);
    return { line: line.from, after: event.clientY > ('y' in box ? box.y : view.documentTop + box.top) + box.height / 2, element };
  };
  const dragOver = (event: DragEvent) => {
    if (!source) return;
    event.preventDefault();
    event.stopPropagation();
    const target = targetAt(event);
    const allowed = source.doc === view.state.doc && target && target.line !== source.line && !view.state.readOnly;
    if (event.dataTransfer) event.dataTransfer.dropEffect = allowed ? 'move' : 'none';
    if (!allowed || !target.element) { clearMarker(); return; }

    // Both sides of a shared line boundary use the next line's leading edge.
    const nextLine = target.after ? target.element.nextElementSibling : null;
    const sharedBoundary = nextLine instanceof HTMLElement && nextLine.matches('.cm-line');
    const element = sharedBoundary ? nextLine : target.element;
    const edge = target.after && !sharedBoundary ? 'after' : 'before';
    const line = view.state.doc.lineAt(target.line);
    const nested = imageDropIndent(line.text, target.after) > 0;
    const inset = nested ? getComputedStyle(target.element).paddingInlineStart : '';
    if (marker === element && marker.dataset.imageDrop === edge && marker.style.getPropertyValue('--image-drop-inset') === inset) return;
    clearMarker();
    marker = element;
    marker.dataset.imageDrop = edge;
    if (inset) marker.style.setProperty('--image-drop-inset', inset);
  };
  const dragLeave = (event: DragEvent) => {
    if (event.relatedTarget instanceof Node && view.dom.contains(event.relatedTarget)) return;
    const rect = view.dom.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX >= rect.right || event.clientY < rect.top || event.clientY >= rect.bottom) clearMarker();
  };
  const drop = (event: DragEvent) => {
    if (!source) return;
    event.preventDefault();
    event.stopPropagation();
    const origin = source;
    clearDrag();
    if (origin.doc !== view.state.doc || view.state.readOnly || !view.contentDOM.isContentEditable) return;
    const target = targetAt(event);
    if (!target) return;
    const text = origin.doc.toString();
    const next = moveImageLine(text, origin.line, target.line, target.after);
    if (next !== text) view.dispatch({ changes: { from: 0, to: text.length, insert: next }, userEvent: 'move.drop' });
    view.focus();
  };
  view.dom.addEventListener('dragstart', dragStart, true);
  view.dom.addEventListener('dragover', dragOver, true);
  view.dom.addEventListener('dragleave', dragLeave);
  view.dom.addEventListener('drop', drop, true);
  view.dom.addEventListener('dragend', clearDrag);
  return () => {
    clearDrag();
    view.dom.removeEventListener('dragstart', dragStart, true);
    view.dom.removeEventListener('dragover', dragOver, true);
    view.dom.removeEventListener('dragleave', dragLeave);
    view.dom.removeEventListener('drop', drop, true);
    view.dom.removeEventListener('dragend', clearDrag);
  };
}
