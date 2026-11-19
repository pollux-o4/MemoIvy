import { WidgetType, type EditorView } from '@codemirror/view';
import expandIcon from '../../../../../src/presentation/icons/actions/media/expand.svg?raw';
import removeIcon from '../../../../../src/presentation/icons/actions/note/trash.svg?raw';
import { deleteImageLine, imageMarkup, MAX_IMAGE_WIDTH, parseImageLine, resizeImageWidth } from './commands';

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
  return parseImageLine(line) ? `${line.match(/^\s*/)?.[0] ?? ''}${markup}` : line;
}

