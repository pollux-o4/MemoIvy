import { WidgetType, type EditorView } from '@codemirror/view';
import expandIcon from '../../../../../src/presentation/icons/actions/media/expand.svg?raw';
import removeIcon from '../../../../../src/presentation/icons/actions/note/trash.svg?raw';
import { deleteImageLine, imageMarkup, MAX_IMAGE_WIDTH, moveImageLine, resizeImageWidth } from './commands';

export interface ImageHandlers {
  view(hash: string): void;
  extract(hash: string, description: string): void;
}

interface AssetWidgetOptions {
  description: string;
  src?: string;
  width?: number;
  hash: string;
  from: number;
  to: number;
  view: EditorView;
  handlers?: ImageHandlers;
}

export class AssetWidget extends WidgetType {
  constructor(readonly options: AssetWidgetOptions) { super(); }

  eq(other: AssetWidget) {
    const a = this.options;
    const b = other.options;
    return a.description === b.description && a.src === b.src && a.width === b.width && a.hash === b.hash
      && a.from === b.from && a.to === b.to && a.view === b.view && a.handlers === b.handlers;
  }

  ignoreEvent(event: Event) { return event.type === 'mousedown'; }

  toDOM() {
    const { description, src, width, hash, from, to, view, handlers } = this.options;
    const wrapper = document.createElement('span');
    wrapper.className = 'cm-asset-preview';
    wrapper.dataset.assetHash = hash;
    wrapper.draggable = !view.state.readOnly && view.contentDOM.isContentEditable;

    let image: HTMLImageElement | undefined;
    if (src) {
      image = document.createElement('img');
      image.src = src;
      image.alt = description || '이미지';
      image.draggable = false;
      image.style.width = width ? `${Math.min(MAX_IMAGE_WIDTH, Math.max(64, width))}px` : 'auto';
      image.style.maxWidth = '100%';
      wrapper.append(image);
    } else {
      const placeholder = document.createElement('div');
      placeholder.textContent = '이미지를 찾을 수 없음';
      placeholder.setAttribute('role', 'img');
      placeholder.setAttribute('aria-label', description || '이미지를 찾을 수 없음');
      wrapper.append(placeholder);
    }

    const open = () => handlers?.view(hash);
    wrapper.addEventListener('dblclick', (event) => { event.preventDefault(); event.stopPropagation(); open(); });
    const tools = document.createElement('span');
    tools.className = 'cm-image-tools';
    tools.append(button('크게 보기', expandIcon, open));
    const remove = button('메모에서 빼기', removeIcon, () => {
      handlers?.extract(hash, description);
      const text = view.state.doc.toString();
      view.dispatch({ changes: { from: 0, to: text.length, insert: deleteImageLine(text, from, to) } });
    });
    remove.disabled = view.state.readOnly || !view.contentDOM.isContentEditable;
    tools.append(remove);
    wrapper.append(tools);

    if (image && wrapper.draggable) {
      const picture = image;
      const handle = document.createElement('button');
      handle.type = 'button';
      handle.className = 'cm-image-resize';
      handle.setAttribute('aria-label', '이미지 크기 조절');
      handle.title = '끌어서 크기 조절 · 두 번 눌러 원래 크기';
      handle.addEventListener('pointerdown', (event) => {
        event.preventDefault();
        event.stopPropagation();
        const rect = picture.getBoundingClientRect();
        const startWidth = rect.width;
        const ratio = rect.height / Math.max(1, rect.width);
        const available = Math.max(64, view.contentDOM.clientWidth);
        const startX = event.clientX;
        const startY = event.clientY;
        const move = (next: PointerEvent) => {
          const nextWidth = resizeImageWidth(startWidth, next.clientX - startX, next.clientY - startY, ratio, available);
          picture.style.width = `${nextWidth}px`;
        };
        const finish = (next: PointerEvent) => {
          handle.removeEventListener('pointermove', move);
          handle.removeEventListener('pointerup', finish);
          handle.removeEventListener('pointercancel', cancel);
          const nextWidth = resizeImageWidth(startWidth, next.clientX - startX, next.clientY - startY, ratio, available);
          if (Math.round(startWidth) !== nextWidth) {
            const current = view.state.doc.toString();
            const line = view.state.doc.lineAt(from);
            const original = current.slice(line.from, line.to);
            const edited = imageMarkup(description, nextWidth, hash);
            view.dispatch({ changes: { from: line.from, to: line.to, insert: replaceImageMarkup(original, edited) } });
          }
        };
        const cancel = () => {
          handle.removeEventListener('pointermove', move);
          handle.removeEventListener('pointerup', finish);
          handle.removeEventListener('pointercancel', cancel);
          picture.style.width = `${width ?? 'auto'}${width ? 'px' : ''}`;
        };
        handle.addEventListener('pointermove', move);
        handle.addEventListener('pointerup', finish);
        handle.addEventListener('pointercancel', cancel);
        handle.setPointerCapture(event.pointerId);
      });
      handle.addEventListener('dblclick', (event) => {
        event.preventDefault();
        event.stopPropagation();
        const current = view.state.doc.toString();
        const line = view.state.doc.lineAt(from);
        const original = current.slice(line.from, line.to);
        view.dispatch({ changes: { from: line.from, to: line.to, insert: replaceImageMarkup(original, imageMarkup(description, undefined, hash)) } });
      });
      wrapper.append(handle);
    }
    return wrapper;
  }
}

export function installImageDropHandlers(view: EditorView) {
  let source: { line: number; doc: EditorView['state']['doc']; element: HTMLElement } | undefined;
  let marker: HTMLElement | undefined;
  const clearMarker = () => {
    marker?.removeAttribute('data-image-drop');
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
    if (marker === element && marker.dataset.imageDrop === edge) return;
    clearMarker();
    marker = element;
    marker.dataset.imageDrop = edge;
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

function button(label: string, icon: string, action: () => void) {
  const element = document.createElement('button');
  element.type = 'button';
  element.className = 'cm-image-tool';
  element.innerHTML = icon.replaceAll('COLOR', 'currentColor');
  element.firstElementChild?.setAttribute('aria-hidden', 'true');
  element.title = label;
  element.setAttribute('aria-label', label);
  element.addEventListener('click', (event) => { event.preventDefault(); event.stopPropagation(); action(); });
  return element;
}

function replaceImageMarkup(line: string, markup: string) {
  return line.replace(/^(\s*)!\[[^\]]*\]\(asset:[a-f\d]{64}\)$/i, `$1${markup}`);
}

