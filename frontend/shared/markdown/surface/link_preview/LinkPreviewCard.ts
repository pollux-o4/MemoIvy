import { linkInfo } from './link_model';

export type PreviewLink = { from: number; to: number; url: string; label: string };

export class LinkPreviewCard {
  readonly element = document.createElement('section');
  onOpen: (url: string) => void = () => {};
  onCopy: (url: string) => void = () => {};
  onEdit: (link: PreviewLink) => void = () => {};

  private readonly badge = document.createElement('span');
  private readonly title = document.createElement('strong');
  private readonly address = document.createElement('div');
  private readonly warning = document.createElement('p');
  private readonly open = button('열기');
  private readonly copy = button('주소 복사');
  readonly edit = button('주소 수정');
  private link: PreviewLink | null = null;

  constructor() {
    this.element.className = 'link-preview-card';
    this.element.hidden = true;
    this.element.setAttribute('role', 'dialog');
    this.badge.className = 'link-preview-badge';
    this.title.className = 'link-preview-title';
    this.address.className = 'link-preview-address';
    this.warning.className = 'link-preview-warning';
    this.open.classList.add('primary');
    this.open.addEventListener('click', () => { if (this.link) this.onOpen(this.link.url); });
    this.copy.addEventListener('click', async () => {
      if (!this.link) return;
      try {
        await navigator.clipboard.writeText(this.link.url);
        this.onCopy(this.link.url);
        this.copy.textContent = '복사됨';
        this.copy.classList.add('confirmed');
        window.setTimeout(() => { this.copy.textContent = '주소 복사'; this.copy.classList.remove('confirmed'); }, 1200);
      } catch { /* Clipboard access can be denied by browser policy. */ }
    });
    this.edit.addEventListener('click', () => { if (this.link) this.onEdit(this.link); });
    const heading = document.createElement('div');
    heading.className = 'link-preview-heading';
    const text = document.createElement('div');
    text.className = 'link-preview-copy';
    text.append(this.title, this.address);
    heading.append(this.badge, text);
    const actions = document.createElement('div');
    actions.className = 'link-preview-actions';
    actions.append(this.open, this.copy, this.edit);
    this.element.append(heading, this.warning, actions);
  }

  update(link: PreviewLink) {
    this.link = link;
    const info = linkInfo(link.url, link.label);
    this.badge.textContent = info.badge;
    this.title.textContent = info.title;
    this.address.textContent = info.address;
    this.address.hidden = !info.address;
    this.address.title = link.url;
    this.warning.textContent = info.warning;
    this.warning.hidden = !info.warning;
    this.open.textContent = info.mail ? '메일 쓰기' : '열기';
    this.edit.hidden = false;
    this.copy.textContent = '주소 복사';
    this.copy.classList.remove('confirmed');
  }
}

function button(label: string) {
  const element = document.createElement('button');
  element.type = 'button';
  element.textContent = label;
  element.className = 'link-preview-action';
  return element;
}
