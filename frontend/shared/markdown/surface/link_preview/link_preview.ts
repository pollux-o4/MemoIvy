import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import type { Extension } from '@codemirror/state';
import { findLinkAt, type LinkTarget } from './link_model';
import { LinkPreviewCard } from './LinkPreviewCard';
import './link_preview.css';

export function linkPreview(handlers: { open(url: string): void }): Extension {
  return ViewPlugin.fromClass(class {
    private readonly card = new LinkPreviewCard();
    private shown: LinkTarget | null = null;
    private pending: number | undefined;
    private hideTimer: number | undefined;
    private pointerOverCard = false;
    private dismissed: LinkTarget | null = null;
    private readonly pointerMove = (event: PointerEvent) => {
      const position = this.view.posAtCoords({ x: event.clientX, y: event.clientY });
      this.track(position, 350, 'pointer');
    };
    private readonly pointerLeave = () => {
      this.clearPending();
      this.scheduleHide();
    };
    private readonly keyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && this.shown) {
        this.dismissed = this.shown;
        this.hide();
      }
    };

    constructor(private readonly view: EditorView) {
      this.card.element.addEventListener('pointerenter', this.onCardEnter);
      this.card.element.addEventListener('pointerleave', this.onCardLeave);
      this.card.onOpen = (url) => { handlers.open(url); this.hide(); };
      this.card.onEdit = (link) => {
        const value = window.prompt('새 주소를 입력하세요.', link.url);
        if (value === null) return;
        this.view.dispatch({ changes: { from: link.from, to: link.to, insert: value } });
        this.hide();
      };
      this.view.dom.addEventListener('pointermove', this.pointerMove);
      this.view.dom.addEventListener('pointerleave', this.pointerLeave);
      this.view.dom.addEventListener('keydown', this.keyDown);
      this.view.scrollDOM.addEventListener('scroll', this.onScroll, { passive: true });
      this.view.dom.append(this.card.element);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.selectionSet) {
        const position = this.view.state.selection.main.head;
        this.track(position, 600, 'caret');
      }
      if (update.geometryChanged) this.place();
    }

    destroy() {
      this.clearPending();
      this.clearHide();
      this.view.dom.removeEventListener('pointermove', this.pointerMove);
      this.view.dom.removeEventListener('pointerleave', this.pointerLeave);
      this.view.dom.removeEventListener('keydown', this.keyDown);
      this.view.scrollDOM.removeEventListener('scroll', this.onScroll);
      this.card.element.remove();
    }

    private readonly onCardEnter = () => { this.pointerOverCard = true; this.clearHide(); };
    private readonly onCardLeave = () => { this.pointerOverCard = false; this.scheduleHide(); };
    private readonly onScroll = () => this.place();

    private track(position: number | null, delay: number, source: 'pointer' | 'caret') {
      if (position === null) return;
      const link = findLinkAt(this.view.state.doc.toString(), position);
      if (!link) {
        if (source === 'caret') this.dismissed = null;
        this.clearPending();
        if (this.shown && !this.pointerOverCard) this.scheduleHide();
        return;
      }
      if (source === 'caret' && this.dismissed) {
        if (sameLink(this.dismissed, link)) return;
        this.dismissed = null;
      }
      if (this.shown && sameLink(this.shown, link)) { this.clearHide(); return; }
      this.clearPending();
      const quickSwitch = source === 'pointer' && Boolean(this.shown);
      this.pending = window.setTimeout(() => {
        this.pending = undefined;
        this.show(link);
      }, quickSwitch ? 60 : delay);
    }

    private show(link: LinkTarget) {
      this.shown = link;
      this.clearHide();
      this.card.update(link);
      this.card.edit.hidden = !this.view.state.facet(EditorView.editable);
      this.place(link.from);
      this.card.element.hidden = false;
    }

    private place(from = this.shown?.from) {
      if (from === undefined) return;
      const coords = this.view.coordsAtPos(from);
      if (!coords) return;
      const bounds = this.view.dom.getBoundingClientRect();
      this.card.element.style.left = `${coords.left - bounds.left}px`;
      this.card.element.style.top = `${coords.bottom - bounds.top + this.view.scrollDOM.scrollTop}px`;
    }

    private scheduleHide() {
      this.clearHide();
      if (this.pointerOverCard) return;
      this.hideTimer = window.setTimeout(() => this.hide(), 220);
    }

    private hide() {
      this.clearPending();
      this.clearHide();
      this.shown = null;
      this.card.element.hidden = true;
    }

    private clearPending() { if (this.pending !== undefined) window.clearTimeout(this.pending); this.pending = undefined; }
    private clearHide() { if (this.hideTimer !== undefined) window.clearTimeout(this.hideTimer); this.hideTimer = undefined; }
  });
}

function sameLink(a: LinkTarget, b: LinkTarget) { return a.from === b.from && a.to === b.to; }
