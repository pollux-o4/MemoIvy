import type { Meta, StoryObj } from '@storybook/react-vite';
import { App } from './App';
import windowKinds from '../../desktop/window_kinds.json';
import { createMockBridge, fixtureNoteIds } from '../shared/mock_bridge';
import type { DesktopBridge, Note, Snapshot } from '../shared/engine_contract';

const assetImage = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const assetUnused = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const makeSnapshot = (): Snapshot => {
  const now = Math.floor(Date.now() / 1000);
  const note: Note = {
    id: '00000000-0000-4000-8000-000000000006',
    text: `# 이미지 참고\n시안에 사용한 이미지입니다.\n\n![미리보기|260](asset:${assetImage})`,
    color: 'purple', modified: now - 900, canvas: { position: [810, 470], next: [], done: false, parent: null },
  };
  return {
    notes: [note],
    assets: [
      { id: assetImage, name: 'reference.png', mime: 'image/png', bytes: 14820, usedBy: [note.id], added: now - 86400 },
      { id: assetUnused, name: 'moodboard.png', mime: 'image/png', bytes: 26904, usedBy: [], added: now - 172800, lastNote: fixtureNoteIds.ideas, unusedSince: now - 86400 },
    ],
    revision: 4, savedRevision: 4, preferences: { expandedProjects: [fixtureNoteIds.project], expandedNotes: [], navigation: {}, autoPrune: false },
  };
};

function storyBridge(snapshot?: Snapshot): DesktopBridge { return createMockBridge(snapshot); }
function previewApp(screen: 'library' | 'note' | 'canvas' | 'files' | 'settings', id?: string, snapshot?: Snapshot, customSize?: number[]) {
  const kind = windowKinds[screen];
  const size = customSize ?? [kind.width, kind.height];
  return <div className="theme-light" style={{ width: size[0], height: size[1], margin: '0 auto', background: 'var(--memoivy-bg)' }}><App bridge={storyBridge(snapshot)} preview={false} initialScreen={screen} initialId={id}/></div>;
}

const meta: Meta<typeof App> = { title: 'MemoIvy/Window states', component: App, parameters: { layout: 'fullscreen' } };
export default meta;
type Story = StoryObj<typeof App>;

export const Library: Story = { render: () => previewApp('library') };
export const Editor: Story = { render: () => previewApp('note', fixtureNoteIds.project) };
export const Canvas: Story = { render: () => previewApp('canvas') };
export const Files: Story = { render: () => previewApp('files', undefined, makeSnapshot()) };
export const Settings: Story = { render: () => previewApp('settings') };
const markdownNoteId = '00000000-0000-4000-8000-000000000007';
const markdownSnapshot = (): Snapshot => ({
  notes: [{ id: markdownNoteId, color: 'blue', modified: Math.floor(Date.now() / 1000), canvas: { position: null, next: [], done: false, parent: null },
    text: '# 큰 제목\n## 리팩토링\n본문에 **굵게**, *기울임*, ~~취소선~~, [링크](https://example.com).\n### 셋째 단계는 본문 크기\n- 글머리 항목\n  Shift+Enter로 이어 쓴 줄\n  - 안쪽 항목\n- [ ] 할 일\n- [x] 끝난 일' }],
  assets: [], revision: 1, savedRevision: 1, preferences: {},
});
const canvasProjectsSnapshot = (): Snapshot => {
  const now = Math.floor(Date.now() / 1000);
  const id = (n: number) => `00000000-0000-4000-8000-0000000001${String(n).padStart(2, '0')}`;
  const node = (position: [number, number] | null, parent: string | null, project = false) => ({ position, next: [], done: false, parent, project });
  const colors = ['blue', 'green', 'pink', 'purple', 'yellow'];
  const children = (parent: string, count: number, offset: number): Note[] => Array.from({ length: count }, (_, i) => ({ id: id(offset + i), text: `# 하위 메모 ${i + 1}`, color: colors[i % colors.length], modified: now - i * 600, canvas: node(null, parent) }));
  return {
    notes: [
      { id: id(1), text: '# 많은 프로젝트', color: 'yellow', modified: now, canvas: node([80, 240], null, true) },
      { id: id(2), text: '# 작은 프로젝트', color: 'green', modified: now, canvas: node([420, 240], null, true) },
      { id: id(3), text: '# 새 프로젝트', color: 'blue', modified: now, canvas: node([760, 240], null, true) },
      ...children(id(1), 5, 10), ...children(id(2), 2, 20),
    ],
    assets: [], revision: 1, savedRevision: 1, preferences: {},
  };
};
export const CanvasProjects: Story = { render: () => previewApp('canvas', undefined, canvasProjectsSnapshot(), [1100, 640]) };
export const MarkdownHeadings: Story = { render: () => previewApp('note', markdownNoteId, markdownSnapshot()) };
export const EmptyLibrary: Story = { render: () => previewApp('library', undefined, { notes: [], assets: [], revision: 0, savedRevision: 0, preferences: {} }) };
export const NarrowWindow: Story = {
  render: () => previewApp('library', undefined, undefined, [360, 600]),
};
export const SaveFailure: Story = {
  render: () => {
    const mock = storyBridge();
    const bridge: DesktopBridge = { ...mock, async request(op, params) {
      if (op === 'update') { const error = new Error('저장 공간에 접근할 수 없습니다.') as Error & { code: string }; error.code = 'IO'; throw error; }
      return mock.request(op, params);
    } };
    return <div className="theme-light" style={{ width: 380, height: 420, margin: '0 auto' }}><App bridge={bridge} preview={false} initialScreen="note" initialId={fixtureNoteIds.ideas}/></div>;
  },
};
