/**
 * GTK 툴팁(menus 기준 표면)을 흉내 낸다. 브라우저 기본 title 툴팁은 운영체제 모양이라 앱과 어긋난다.
 * title을 data-tip으로 옮겨 기본 툴팁을 막고, 이름이 없던 요소는 aria-label로 이름을 보존한다.
 */
const DELAY_MS = 500;

export function installTooltips(root: Document = document): () => void {
  const bubble = root.createElement('div');
  bubble.className = 'app-tooltip';
  bubble.setAttribute('role', 'tooltip');
  root.body.append(bubble);
  let timer: number | undefined;
  let owner: HTMLElement | null = null;

  const hide = () => { window.clearTimeout(timer); owner = null; bubble.classList.remove('is-visible'); };
  const adopt = (element: HTMLElement) => {
    const title = element.getAttribute('title');
    if (!title) return;
    element.dataset.tip = title;
    element.removeAttribute('title');
    if (!element.hasAttribute('aria-label') && !element.textContent?.trim()) element.setAttribute('aria-label', title);
  };
  const show = (element: HTMLElement) => {
    bubble.textContent = element.dataset.tip ?? '';
    const rect = element.getBoundingClientRect();
    bubble.classList.add('is-visible');
    const width = bubble.offsetWidth;
    const left = Math.min(Math.max(6, rect.left + rect.width / 2 - width / 2), window.innerWidth - width - 6);
    const below = rect.bottom + 6 + bubble.offsetHeight < window.innerHeight;
    bubble.style.left = `${left}px`;
    bubble.style.top = `${below ? rect.bottom + 6 : rect.top - bubble.offsetHeight - 6}px`;
  };
  const over = (event: PointerEvent) => {
    const element = (event.target as Element | null)?.closest<HTMLElement>('[title], [data-tip]');
    if (!element || element === owner) return;
    hide();
    adopt(element);
    if (!element.dataset.tip) return;
    owner = element;
    timer = window.setTimeout(() => { if (owner === element && element.isConnected) show(element); }, DELAY_MS);
  };
  const out = (event: PointerEvent) => { if (owner && !owner.contains(event.relatedTarget as Node | null)) hide(); };

  root.addEventListener('pointerover', over);
  root.addEventListener('pointerout', out);
  root.addEventListener('pointerdown', hide, true);
  root.addEventListener('keydown', hide, true);
  root.addEventListener('scroll', hide, true);
  return () => {
    hide(); bubble.remove();
    root.removeEventListener('pointerover', over); root.removeEventListener('pointerout', out);
    root.removeEventListener('pointerdown', hide, true); root.removeEventListener('keydown', hide, true); root.removeEventListener('scroll', hide, true);
  };
}
