import { WidgetType, type EditorView } from '@codemirror/view';
import { deleteImageLine, displayImageSize, imageMarkup, MAX_IMAGE_WIDTH, moveImageLine, resizeImageWidth } from './commands';

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

  ignoreEvent() { return false; }

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
    wrapper.append(button('크게 보기', open));
    const remove = button('메모에서 빼기', () => {
      handlers?.extract(hash, description);
      const text = view.state.doc.toString();
      view.dispatch({ changes: { from: 0, to: text.length, insert: deleteImageLine(text, from, to) } });
    });
    remove.disabled = view.state.readOnly || !view.contentDOM.isContentEditable;
    wrapper.append(remove);

    if (image && wrapper.draggable) {
      const handle = document.createElement('button');
      handle.type = 'button';
      handle.className = 'cm-image-resize';
      handle.setAttribute('aria-label', '이미지 크기 조절');
      handle.title = '끌어서 크기 조절 · 두 번 눌러 원래 크기';
      handle.addEventListener('pointerdown', (event) => {
        event.preventDefault();
        event.stopPropagation();
        const rect = image!.getBoundingClientRect();
        const startWidth = rect.width;
        const ratio = rect.height / Math.max(1, rect.width);
        const available = Math.max(64, view.contentDOM.clientWidth);
        const startX = event.clientX;
        const startY = event.clientY;
        const move = (next: PointerEvent) => {
          const nextWidth = resizeImageWidth(startWidth, next.clientX - startX, next.clientY - startY, ratio, available);
          image!.style.width = `${nextWidth}px`;
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
          image!.style.width = `${width ?? 'auto'}${width ? 'px' : ''}`;
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
  const dragStart = (event: DragEvent) => {
    const element = (event.target as HTMLElement | null)?.closest<HTMLElement>('.cm-asset-preview');
    const hash = element?.dataset.assetHash;
    if (!hash || !event.dataTransfer) return;
    const from = view.posAtDOM(element!);
    const line = view.state.doc.lineAt(from);
    const match = /^\s*!\[([^\]]*)\]\(asset:([a-f\d]{64})\)$/i.exec(line.text);
    if (!match) return;
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('application/x-memoivy-image', String(line.from));
    event.dataTransfer.setData('text/plain', imageMarkup(match[1].replace(/\|\d+$/, ''), Number(/\|(\d+)/.exec(match[1])?.[1]) || undefined, match[2]));
  };
  const dragOver = (event: DragEvent) => {
    if (event.dataTransfer?.types.includes('application/x-memoivy-image')) event.preventDefault();
  };
  const drop = (event: DragEvent) => {
    const source = Number(event.dataTransfer?.getData('application/x-memoivy-image'));
    if (!Number.isInteger(source) || !event.dataTransfer?.types.includes('application/x-memoivy-image')) return;
    event.preventDefault();
    if (view.state.readOnly || !view.contentDOM.isContentEditable) return;
    const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (position == null) return;
    const text = view.state.doc.toString();
    const target = view.state.doc.lineAt(position).from;
    const next = moveImageLine(text, source, target);
    if (next !== text) view.dispatch({ changes: { from: 0, to: text.length, insert: next } });
  };
  view.dom.addEventListener('dragstart', dragStart);
  view.dom.addEventListener('dragover', dragOver);
  view.dom.addEventListener('drop', drop);
  return () => {
    view.dom.removeEventListener('dragstart', dragStart);
    view.dom.removeEventListener('dragover', dragOver);
    view.dom.removeEventListener('drop', drop);
  };
}

function button(label: string, action: () => void) {
  const element = document.createElement('button');
  element.type = 'button';
  element.className = 'cm-image-tool';
  element.textContent = label;
  element.setAttribute('aria-label', label);
  element.addEventListener('click', (event) => { event.preventDefault(); event.stopPropagation(); action(); });
  return element;
}

function replaceImageMarkup(line: string, markup: string) {
  return line.replace(/^(\s*)!\[[^\]]*\]\(asset:[a-f\d]{64}\)$/i, `$1${markup}`);
}

