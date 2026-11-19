import { linkInfo, type LinkTarget } from './link_model';

export class LinkPreviewCard {
  readonly element = document.createElement('section');
  onOpen: (url: string) => void = () => {};
  onCopy: (url: string) => void = () => {};
  onSave: (link: LinkTarget, value: string) => void = () => {};
  onCancel: () => void = () => {};

  private readonly badge = document.createElement('span');
  private readonly title = document.createElement('strong');
  private readonly address = document.createElement('div');
  private readonly warning = document.createElement('p');
  private readonly open = button('열기');
  private readonly copy = button('주소 복사');
  readonly edit = button('주소 수정');
  private readonly input = document.createElement('input');
  private readonly actions = document.createElement('div');
  private readonly editActions = document.createElement('div');
  private link: LinkTarget | null = null;

  get editing() { return !this.input.hidden; }

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
    const form = document.createElement('form');
    this.input.className = 'link-preview-input';
    this.input.setAttribute('aria-label', '링크 주소');
    this.input.inputMode = 'url';
    this.input.spellcheck = false;
    this.input.required = true;
    this.input.pattern = String.raw`[^\s<>\(\)]+`;
    const save = button('저장');
    save.type = 'submit';
    save.classList.add('primary');
    const cancel = button('취소');
    cancel.addEventListener('click', () => this.onCancel());
    this.edit.addEventListener('click', () => {
      if (!this.link) return;
      this.input.value = this.link.url;
      this.setEditing(true);
      this.input.focus();
      this.input.select();
    });
    form.addEventListener('submit', (event) => {
      event.preventDefault();
      if (this.editing && this.link) this.onSave(this.link, this.input.value);
    });
    const heading = document.createElement('div');
    heading.className = 'link-preview-heading';
    const text = document.createElement('div');
    text.className = 'link-preview-copy';
    text.append(this.title, this.address, this.input);
    heading.append(this.badge, text);
    this.actions.className = this.editActions.className = 'link-preview-actions';
    this.actions.append(this.open, this.copy, this.edit);
    this.editActions.append(cancel, save);
    form.append(heading, this.warning, this.actions, this.editActions);
    this.element.append(form);
    this.setEditing(false);
  }

  private setEditing(editing: boolean) {
    this.input.hidden = this.editActions.hidden = !editing;
    this.address.hidden = editing || !this.address.textContent;
    this.actions.hidden = editing;
  }

  update(link: LinkTarget) {
    this.link = link;
    const info = linkInfo(link.url, link.label);
    this.badge.textContent = info.badge;
    this.title.textContent = info.title;
    this.address.textContent = info.address;
    this.setEditing(false);
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
