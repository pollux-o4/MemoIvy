import { Decoration, WidgetType, type EditorView } from '@codemirror/view';
import { blockLineAt, createBlockScanner, type ListItem as Item, type TaskState, taskBoxOffset } from '../list_item';

/** GTK live_editor: 표식 칸 24px(깊이마다 추가), 표식과 본문 사이 6px, 글머리표 •◦▪. */
const GUTTER = 24;
const BULLETS = ['•', '◦', '▪'] as const;
type Range = { from: number; to: number; value: Decoration };

abstract class MarkerWidget extends WidgetType {
  constructor(readonly depth: number) { super(); }
  protected slot(content: HTMLElement) {
    const slot = document.createElement('span');
    slot.className = 'md-marker-slot';
    slot.style.left = `${GUTTER * this.depth}px`;
    slot.append(content);
    return slot;
  }
  ignoreEvent() { return false; }
}

class BulletWidget extends MarkerWidget {
  constructor(depth: number, readonly label?: string) { super(depth); }
  eq(other: WidgetType) { return other instanceof BulletWidget && other.depth === this.depth && other.label === this.label; }
  toDOM() {
    const glyph = document.createElement('span');
    glyph.className = 'md-bullet';
    glyph.textContent = this.label ?? BULLETS[Math.min(this.depth, BULLETS.length - 1)];
    glyph.setAttribute('aria-hidden', 'true');
    return this.slot(glyph);
  }
}

const TASK_VIEW: Record<TaskState, { className: string; checked: string; label: string }> = {
  ' ': { className: '', checked: 'false', label: '할 일' },
  x: { className: ' is-checked', checked: 'true', label: '할 일 완료됨' },
  '/': { className: ' is-doing', checked: 'mixed', label: '진행 중인 할 일' },
};
/** 슬롯이 지금 그리는 위젯. 상태만 바뀌면 DOM을 재사용하므로 클릭은 최신 위젯을 읽는다. */
const currentTask = new WeakMap<HTMLElement, TaskWidget>();

/**
 * 체크박스는 상태가 바뀌어도 같은 DOM을 고쳐 쓴다(updateDOM). 새로 그리면 CSS 전이가 일어나지 않아 완료 모션이 끊긴다.
 */
class TaskWidget extends MarkerWidget {
  constructor(depth: number, readonly state: TaskState, readonly box: number) { super(depth); }
  eq(other: WidgetType) { return other instanceof TaskWidget && other.depth === this.depth && other.state === this.state && other.box === this.box; }
  toDOM(view: EditorView) {
    const box = document.createElement('button');
    box.type = 'button';
    box.setAttribute('role', 'checkbox');
    box.addEventListener('mousedown', (event) => event.preventDefault());
    const slot = this.slot(box);
    box.addEventListener('click', (event) => {
      event.preventDefault();
      const widget = currentTask.get(slot);
      if (view.composing || !widget) return;
      const next = event.shiftKey ? (widget.state === '/' ? ' ' : '/') : widget.state === 'x' ? ' ' : 'x';
      view.dispatch({ changes: { from: widget.box, to: widget.box + 3, insert: `[${next}]` }, scrollIntoView: false });
      view.focus();
    });
    this.apply(slot);
    return slot;
  }
  updateDOM(dom: HTMLElement) {
    if (!currentTask.has(dom) || currentTask.get(dom)?.depth !== this.depth) return false;
    this.apply(dom);
    return true;
  }
  private apply(slot: HTMLElement) {
    currentTask.set(slot, this);
    const box = slot.firstElementChild as HTMLElement;
    const view = TASK_VIEW[this.state];
    box.className = `md-checkbox${view.className}`;
    box.setAttribute('aria-checked', view.checked);
    box.setAttribute('aria-label', view.label);
  }
}

type Line = { from: number; text: string };

function itemAtLine(state: EditorView['state'], number: number) {
  const block = blockLineAt((current) => state.doc.line(current).text, number);
  return block.code ? null : block.owner;
}
const lineStyle = (depth: number, extra = '') => Decoration.line({ class: `md-list${extra}`, attributes: { style: `padding-left: ${GUTTER * (depth + 1)}px` } });

/**
 * 목록 줄은 기호를 숨기고 표식 칸에 글머리표·체크박스를 그린다(커서 줄에서도 유지, GTK와 동일).
 * 항목 아래 표식 너비 이상 들여쓴 줄은 같은 항목의 이어쓰기로 보고, 공백을 숨겨 항목 글자와 같은 칸에서 시작한다.
 */
export function createListScanner() {
  const scan = createBlockScanner();
  return (line: Line): Range[] => {
    const { item, owner, code } = scan(line.text);
    if (code) return [];
    if (item) {
      const widget = item.task !== undefined
        ? new TaskWidget(item.depth, item.task, line.from + taskBoxOffset(item))
        : new BulletWidget(item.depth, item.ordered ? item.marker.trim() : undefined);
      return [
        { from: line.from, to: line.from, value: lineStyle(item.depth, item.task === 'x' ? ' md-task-done' : '') },
        { from: line.from, to: line.from + item.markerEnd, value: Decoration.replace({ widget }) },
      ];
    }
    const current = owner?.item;
    if (!current || line.text.length - line.text.trimStart().length < current.contentIndent) return [];
    return [
      { from: line.from, to: line.from, value: lineStyle(current.depth) },
      { from: line.from, to: line.from + current.contentIndent, value: Decoration.replace({}) },
    ];
  };
}

/** GTK list_edit: 목록 항목에서 Shift+Enter는 같은 항목 안에서 줄을 바꾼다(표식 너비만큼 들여쓴 이어쓰기 줄). */
export function continueListItem(view: EditorView): boolean {
  if (view.composing) return false;
  const { state } = view;
  const head = state.selection.main.head;
  const selection = state.selection.main;
  if (!selection.empty) {
    const fromLine = state.doc.lineAt(selection.from);
    const toLine = state.doc.lineAt(selection.to);
    const fromItem = itemAtLine(state, fromLine.number);
    const toItem = itemAtLine(state, toLine.number);
    if (!fromItem || !toItem || fromItem.markerLine !== toItem.markerLine) return false;
    if (fromLine.number === fromItem.markerLine && selection.from < fromLine.from + fromItem.item.contentIndent) return false;
  }
  const context = itemAtLine(state, state.doc.lineAt(head).number);
  if (!context) return false;
  view.dispatch(state.replaceSelection(`\n${' '.repeat(context.item.contentIndent)}`), { scrollIntoView: true, userEvent: 'input' });
  return true;
}

export function nextListMarker(item: Item) {
  if (item.ordered) return `${item.marker.slice(0, item.marker.length - `${item.number}${item.delimiter} `.length)}${(item.number ?? 0) + 1}${item.delimiter} `;
  const bullet = item.marker.match(/[-*+]/)?.[0] ?? '-';
  return `${' '.repeat(item.depth * 2)}${bullet} ${item.task === undefined ? '' : '[ ] '}`;
}

/** GTK list_edit 키 동작. 원문 변경은 CodeMirror 한 transaction으로 undo 한 번에 묶는다. */
export function handleListKey(view: EditorView, key: string): boolean {
  if (view.composing) return false;
  const { state } = view;
  const selection = state.selection.main;
  const line = state.doc.lineAt(selection.head);
  const scan = createBlockScanner();
  const contexts = Array.from({ length: state.doc.lines }, (_, index) => scan(state.doc.line(index + 1).text));
  const context = contexts[line.number - 1].code ? null : contexts[line.number - 1].owner;
  if (!context) return false;
  const { item, markerLine } = context;

  if (key === 'Home') {
    const target = line.from + item.contentIndent;
    view.dispatch({ selection: { anchor: target, head: target } });
    return true;
  }
  if (key === 'ArrowLeft' && !selection.empty && selection.from !== line.from + item.contentIndent) return false;
  if (key === 'ArrowLeft' && selection.empty && line.number === markerLine && selection.head === line.from + item.contentIndent) {
    if (line.number > 1) view.dispatch({ selection: { anchor: state.doc.line(line.number - 1).to } });
    return true;
  }
  if (key === 'Tab' || key === 'Shift-Tab') {
    const first = state.doc.lineAt(selection.from).number;
    const last = state.doc.lineAt(selection.to).number;
    const roots: Array<{ number: number; item: Item }> = [];
    for (let number = first; number <= last; number++) {
      const parsed = contexts[number - 1].item;
      if (parsed) roots.push({ number, item: parsed });
    }
    if (!roots.length && selection.empty) roots.push({ number: markerLine, item });
    const included = new Set<number>();
    for (const root of roots) {
      if (included.has(root.number)) continue;
      included.add(root.number);
      for (let number = root.number + 1; number <= state.doc.lines; number++) {
        const child = contexts[number - 1].item;
        if (child && child.depth <= root.item.depth) break;
        if (!child && !contexts[number - 1].owner) break;
        included.add(number);
      }
    }
    const changes = [...included].sort((a, b) => a - b).flatMap((number) => {
      const current = state.doc.line(number);
      if (key === 'Tab') return [{ from: current.from, insert: '  ' }];
      if (current.text.startsWith('  ')) return [{ from: current.from, to: current.from + 2, insert: '' }];
      if (current.text.startsWith('\t')) return [{ from: current.from, to: current.from + 1, insert: '' }];
      return [];
    });
    if (!changes.length) return false;
    view.dispatch({ changes, userEvent: 'input' });
    return true;
  }
  if (!selection.empty) return false;

  if (key === 'Enter') {
    if (!line.text.slice(item.contentIndent).trim()) {
      if (item.depth > 0) view.dispatch({ changes: { from: line.from, to: line.from + 2, insert: '' }, userEvent: 'input' });
      else view.dispatch({ changes: { from: line.from, to: line.from + item.contentIndent, insert: '' }, userEvent: 'input' });
      return true;
    }
    const marker = nextListMarker(item);
    const changes: { from: number; to?: number; insert: string }[] = [{ from: selection.head, insert: `\n${marker}` }];
    if (item.ordered) {
      let number = (item.number ?? 0) + 2;
      for (let following = markerLine + 1; following <= state.doc.lines; following++) {
        const nextLine = state.doc.line(following);
        const followingContext = contexts[following - 1];
        if (!followingContext.owner) break;
        const next = followingContext.item;
        if (!next) continue;
        if (next.depth < item.depth) break;
        if (next.depth > item.depth) continue;
        if (!next.ordered || followingContext.owner.sequence !== context.sequence) break;
        const digits = String(next.number ?? 0);
        if (next.number !== number) changes.push({ from: nextLine.from + next.marker.indexOf(digits), to: nextLine.from + next.marker.indexOf(digits) + digits.length, insert: String(number) });
        number++;
      }
    }
    view.dispatch({ changes, selection: { anchor: selection.head + 1 + marker.length }, scrollIntoView: true, userEvent: 'input' });
    return true;
  }
  if (key === 'Backspace' && selection.head === line.from + item.contentIndent) {
    const remove = item.depth > 0 ? Math.min(2, item.marker.length - item.marker.trimStart().length) : item.contentIndent;
    if (remove) view.dispatch({ changes: { from: line.from, to: line.from + remove, insert: '' }, userEvent: 'input' });
    return true;
  }
  if (key === 'Delete' && selection.head === line.to && line.number < state.doc.lines) {
    const following = state.doc.line(line.number + 1);
    const next = contexts[line.number].item;
    if (!next) return false;
    view.dispatch({ changes: { from: line.to, to: following.from + next.contentIndent, insert: '' }, userEvent: 'input' });
    return true;
  }
  return false;
}
