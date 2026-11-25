import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import { App } from './App';
import { createMockBridge, fixtureNoteIds } from '../shared/mock_bridge';
import type { DesktopBridge, Note, Snapshot } from '../shared/engine_contract';

const projectId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1';
const childId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2';
const now = 1_794_576_000;
const projectSnapshot = (): Snapshot => {
  const project: Note = { id: projectId, text: '# 테스트 프로젝트\n하위 메모를 펼칩니다.', color: 'yellow', modified: now, canvas: { position: [0, 0], next: [], done: false, parent: null, project: true } };
  const child: Note = { id: childId, text: '# 하위 메모\n프로젝트 안의 메모입니다.', color: 'blue', modified: now, canvas: { position: [0, 0], next: [], done: false, parent: projectId } };
  return { notes: [project, child], assets: [], revision: 1, savedRevision: 1, preferences: { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false } };
};
const librarySnapshot = (): Snapshot => {
  const note: Note = { id: childId, text: '# 키보드 포커스 카드\nTab으로 검색에서 이동합니다.', color: 'blue', modified: now, canvas: { position: [0, 0], next: [], done: false, parent: null } };
  return { notes: [note], assets: [], revision: 1, savedRevision: 1, preferences: { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false } };
};
const frame = (bridge: DesktopBridge, screen: 'library' | 'note' | 'canvas', id?: string, size: [number, number] = [440, 600]) => <div className="theme-light" style={{ width: size[0], height: size[1], margin: '0 auto' }}><App bridge={bridge} preview={false} initialScreen={screen} initialId={id}/></div>;

const meta: Meta<typeof App> = { title: 'Interactions/App', component: App, parameters: { layout: 'fullscreen' } };
export default meta;
type Story = StoryObj<typeof App>;

export const ProjectDisclosure: Story = {
  render: () => frame(createMockBridge(projectSnapshot()), 'library'),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const disclosure = await canvas.findByRole('button', { name: '테스트 프로젝트 펼치기' });
    await expect(disclosure).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(disclosure);
    await expect(disclosure).toHaveAttribute('aria-expanded', 'true');
    await expect(await canvas.findByRole('button', { name: /^하위 메모(?! (펼치기|접기|메뉴))/ })).toBeVisible();
    await userEvent.click(disclosure);
    await expect(disclosure).toHaveAttribute('aria-expanded', 'false');
  },
};

export const KeyboardFocus: Story = {
  render: () => frame(createMockBridge(librarySnapshot()), 'library'),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const search = await canvas.findByRole('textbox', { name: '메모 검색' });
    const card = await canvas.findByRole('button', { name: /^키보드 포커스 카드(?! (펼치기|접기|메뉴))/ });
    await expect(search).toHaveFocus();
    await userEvent.tab();
    await expect(card).toHaveFocus();
  },
};

export const SaveFailure: Story = {
  render: () => {
    const mock = createMockBridge();
    const bridge: DesktopBridge = { ...mock, async request(op, params) {
      if (op === 'update') { const error = new Error('저장 공간에 접근할 수 없습니다.') as Error & { code: string }; error.code = 'IO'; throw error; }
      return mock.request(op, params);
    } };
    return frame(bridge, 'note', fixtureNoteIds.ideas);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByText('저장됨');
    // 편집기는 지연 로딩이라 저장 상태보다 늦게 붙는다.
    const editor = await waitFor(() => {
      const surface = canvasElement.querySelector<HTMLElement>('.cm-content[contenteditable="true"]');
      if (!surface) throw new Error('Editor content surface was not mounted.');
      return surface;
    });
    const originalLines = [...editor.querySelectorAll('.cm-line')].map((line) => line.textContent);
    await userEvent.click(editor);
    await userEvent.keyboard('{Control>}{End}{/Control}');
    // 글자별 DOM 변경은 CodeMirror의 선택 영역 동기화와 충돌하므로 편집기가 처리하는 paste 경로를 쓴다.
    await userEvent.paste('\n상태 확인용 초안');
    await expect(await canvas.findByText('저장하지 못했어요. 저장 공간에 접근할 수 없습니다.')).toBeVisible();
    await expect([...editor.querySelectorAll('.cm-line')].map((line) => line.textContent)).toEqual([...originalLines, '상태 확인용 초안']);
  },
};

/** CANVAS-003 정렬 막대: 첫 면은 묶음만, 묶음을 열면 같은 막대가 동작으로 바뀌고 Esc는 한 단계만 올라간다. */
export const AlignmentBarMorph: Story = {
  render: () => frame(createMockBridge(), 'canvas', undefined, [1000, 720]),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findAllByRole('article');
    canvasElement.querySelector<HTMLElement>('.canvas-screen')?.focus();
    await userEvent.keyboard('{Control>}a{/Control}');
    const bar = await canvas.findByRole('toolbar', { name: '선택 항목 정렬' });
    await expect(within(bar).queryByRole('button', { name: '왼쪽 맞춤' })).toBeNull();
    await userEvent.click(within(bar).getByRole('button', { name: /맞춤/ }));
    const align = await canvas.findByRole('toolbar', { name: '선택 항목 맞춤' });
    await expect(within(align).getByRole('button', { name: '왼쪽 맞춤' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    const back = await canvas.findByRole('toolbar', { name: '선택 항목 정렬' });
    await expect(within(back).getByRole('button', { name: /맞춤/ })).toHaveFocus();
  },
};

const inspectorSnapshot = (): Snapshot => {
  const base = projectSnapshot();
  const memo: Note = { id: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb4', text: '# 독립 메모\n- [ ] 할 일', color: 'blue', modified: now, canvas: { position: [320, 0], next: [], done: false, parent: null } };
  return { ...base, notes: [...base.notes, memo] };
};
const selectCard = async (root: HTMLElement, title: string) => {
  const card = [...root.querySelectorAll<HTMLElement>('[data-note-id]')].find((node) => node.textContent?.includes(title));
  if (!card) throw new Error(`카드 없음: ${title}`);
  await userEvent.click(card);
  const inspector = await waitFor(() => {
    const found = root.querySelector<HTMLElement>('.canvas-inspector');
    if (!found) throw new Error('인스펙터가 열리지 않음');
    return found;
  });
  return within(inspector);
};

/** 인스펙터 주 동작: 프로젝트는 들어가기 + 보조 '메모로 열기', 메모는 '메모 열기' 하나. 단축키 칩은 한 모양. */
export const InspectorActions: Story = {
  render: () => frame(createMockBridge(inspectorSnapshot()), 'canvas', undefined, [900, 620]),
  play: async ({ canvasElement }) => {
    await within(canvasElement).findAllByRole('article');
    const project = await selectCard(canvasElement, '테스트 프로젝트');
    await expect(await project.findByRole('button', { name: '프로젝트 들어가기' })).toBeVisible();
    await expect(project.getByRole('button', { name: '메모로 열기' })).toBeVisible();
    // 본문은 메모와 같은 미리보기이고, 누르면 같은 자리에서 소개 편집기로 바뀐다.
    await userEvent.click(project.getByRole('button', { name: '프로젝트 소개 고치기' }));
    await expect(await project.findByRole('textbox', { name: '프로젝트 소개' })).toHaveFocus();
    const memo = await selectCard(canvasElement, '독립 메모');
    await expect(await memo.findByRole('button', { name: '메모 열기' })).toBeVisible();
    await expect(memo.queryByRole('button', { name: '메모로 열기' })).toBeNull();
    const chips = canvasElement.querySelectorAll('.canvas-inspector kbd');
    await expect([...chips].every((chip) => chip.classList.contains('inspector-kbd'))).toBe(true);
  },
};

const taskId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb3';
const taskSnapshot = (): Snapshot => {
  const note: Note = { id: taskId, text: '# 오늘 할 일\n- [ ] 장보기\n- [x] 메일 답장', color: 'green', modified: now, canvas: { position: [0, 0], next: [], done: false, parent: null } };
  return { notes: [note], assets: [], revision: 1, savedRevision: 1, preferences: { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false } };
};

/** 할 일 완료 모션: 체크박스는 같은 요소가 상태만 바꿔야 칠하기·체크 긋기·글자 흐려짐 전이가 이어진다. */
export const TaskCompletion: Story = {
  render: () => frame(createMockBridge(taskSnapshot()), 'note', taskId),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const [box] = await canvas.findAllByRole('checkbox', { name: '할 일' });
    await userEvent.click(box);
    await expect(box).toHaveAttribute('aria-checked', 'true');
    await expect(box.isConnected).toBe(true);
    await expect(box.closest('.cm-line')).toHaveClass('md-task-done');
    await userEvent.click(box);
    await expect(box).toHaveAttribute('aria-checked', 'false');
  },
};

const doneProjectId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb4';
const doneChildId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb5';
const nearlyDone = { project: '# 출시 준비\n- [x] 문서 정리', child: '# 점검\n- [x] 테스트\n- [ ] 배포' };
const completionSnapshot = (): Snapshot => {
  const project: Note = { id: doneProjectId, text: nearlyDone.project, color: 'green', modified: now, canvas: { position: [80, 80], next: [], done: false, parent: null, project: true } };
  const child: Note = { id: doneChildId, text: nearlyDone.child, color: 'yellow', modified: now, canvas: { position: [0, 0], next: [], done: false, parent: doneProjectId } };
  return { notes: [project, child], assets: [], revision: 1, savedRevision: 1, preferences: { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false } };
};
/** 마지막 할 일을 끝내 100%에 닿게 한다. 다시 보려면 window.memoivyReopenLast() 뒤 memoivyCompleteLast()를 부른다. */
const completionStory = (screen: 'library' | 'canvas', size: [number, number]): Story => {
  const bridge = createMockBridge(completionSnapshot());
  // 다른 창에서 고친 것처럼 최신 리비전을 기준으로 저장한다.
  const write = async (text: string) => {
    const { snapshot } = await bridge.request('snapshot') as { snapshot: Snapshot };
    await bridge.request('update', { id: doneChildId, text, baseRevision: snapshot.revision });
  };
  const completeLast = () => write(nearlyDone.child.replace('- [ ] 배포', '- [x] 배포'));
  const reopenLast = () => write(nearlyDone.child);
  return {
    render: () => { Object.assign(window, { memoivyCompleteLast: completeLast, memoivyReopenLast: reopenLast }); return frame(bridge, screen, undefined, size); },
    play: async ({ canvasElement }) => {
      const bar = await within(canvasElement).findByRole('progressbar', { name: '출시 준비 진행률' });
      await expect(bar).toHaveAttribute('aria-valuenow', '1');
      await completeLast();
      await expect(await within(canvasElement).findByRole('progressbar', { name: '출시 준비 진행률' })).toHaveAttribute('aria-valuenow', '2');
    },
  };
};
/** 진행률 100% 순간: 막대가 다 찬 뒤 빛이 한 번 지나가고 개수 표시가 살짝 튄다(목록형). */
export const ProjectCompleteLibrary: Story = completionStory('library', [440, 360]);
/** 같은 순간을 캔버스 프로젝트 카드에서 본다. */
export const ProjectCompleteCanvas: Story = completionStory('canvas', [640, 420]);
