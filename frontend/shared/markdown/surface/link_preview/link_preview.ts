import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import type { Extension } from '@codemirror/state';
import { externalLinkUrl, findLinkAt, type LinkTarget } from './link_model';
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
      if (this.pointerOverCard || (this.shown && this.card.editing)) return;
      const position = this.view.posAtCoords({ x: event.clientX, y: event.clientY });
      this.track(position, 350, 'pointer');
    };
    private readonly pointerLeave = () => {
      this.clearPending();
      this.scheduleHide();
    };
    private readonly keyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.isComposing && this.shown) {
        if (this.card.editing) {
          event.preventDefault();
          event.stopPropagation();
          this.finishEditing();
          return;
        }
        this.dismissed = this.shown;
        this.hide();
      }
    };

    constructor(private readonly view: EditorView) {
      this.card.element.addEventListener('pointerenter', this.onCardEnter);
      this.card.element.addEventListener('pointerleave', this.onCardLeave);
      this.card.onOpen = (url) => { handlers.open(externalLinkUrl(url)); this.hide(); };
      this.card.onCancel = () => this.finishEditing();
      this.card.onSave = (link, value) => {
        this.finishEditing();
        const url = externalLinkUrl(value);
        if (link.url !== url) {
          this.view.dispatch({ changes: { from: link.from, to: link.to, insert: url } });
        }
      };
      this.view.dom.addEventListener('pointermove', this.pointerMove);
      this.view.dom.addEventListener('pointerleave', this.pointerLeave);
      this.view.dom.addEventListener('keydown', this.keyDown);
      this.view.scrollDOM.addEventListener('scroll', this.onScroll, { passive: true });
      this.view.dom.append(this.card.element);
    }

    update(update: ViewUpdate) {
      if (this.shown && this.card.editing) {
        if (update.docChanged || !update.state.facet(EditorView.editable)) this.hide();
        else if (update.geometryChanged) this.place();
        return;
      }
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

    private readonly onCardEnter = () => {
      this.pointerOverCard = true;
      this.clearPending();
      this.clearHide();
    };
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
      if (this.shown && sameLink(this.shown, link)) {
        this.clearPending();
        this.clearHide();
        return;
      }
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
      this.place();
    }

    private place() {
      if (!this.shown) return;
      this.view.requestMeasure({
        key: this,
        read: (view) => {
          if (!this.shown) return null;
          const coords = view.coordsAtPos(this.shown.from);
          if (!coords) return null;
          const bounds = view.dom.getBoundingClientRect();
          const width = Math.min(360, bounds.width - 16);
          const left = Math.max(8, Math.min(coords.left - bounds.left, bounds.width - width - 8));
          return { left, top: coords.bottom - bounds.top };
        },
        write: (position) => {
          if (!position) return;
          this.card.element.style.left = `${position.left}px`;
          this.card.element.style.top = `${position.top}px`;
          this.card.element.hidden = false;
        },
      });
    }

    private finishEditing() {
      this.dismissed = this.shown;
      this.hide();
      this.view.focus();
    }

    private scheduleHide() {
      this.clearHide();
      if (this.pointerOverCard || (this.shown && this.card.editing)) return;
      this.hideTimer = window.setTimeout(() => this.hide(), 220);
    }

    private hide() {
      this.clearPending();
      this.clearHide();
      this.shown = null;
      this.pointerOverCard = false;
      this.card.element.hidden = true;
    }

    private clearPending() { if (this.pending !== undefined) window.clearTimeout(this.pending); this.pending = undefined; }
    private clearHide() { if (this.hideTimer !== undefined) window.clearTimeout(this.hideTimer); this.hideTimer = undefined; }
  });
}

function sameLink(a: LinkTarget, b: LinkTarget) { return a.from === b.from && a.to === b.to; }
