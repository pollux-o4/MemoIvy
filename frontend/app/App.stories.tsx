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

const meta: Meta<typeof App> = { title: 'Screens/Window States', component: App, parameters: { layout: 'fullscreen' } };
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
      { id: id(2), text: '# 작은 프로젝트\n안녕하세요\n- [ ] 위 할 일\n  - [ ] 상세 할 일\n  - [ ] 또 다른 할 일', color: 'green', modified: now, canvas: node([420, 240], null, true) },
      { id: id(3), text: '# 새 프로젝트', color: 'blue', modified: now, canvas: node([760, 240], null, true) },
      ...children(id(1), 5, 10), ...children(id(2), 2, 20),
    ],
    assets: [], revision: 1, savedRevision: 1, preferences: {},
  };
};
export const CanvasProjects: Story = { render: () => previewApp('canvas', undefined, canvasProjectsSnapshot(), [1100, 640]) };
/** 가지 두 개(위: 기획 → 시안·조사, 아래: 서로 이어진 한 쌍). 카드를 고르면 그 가지만 또렷해진다. */
const canvasBranchesSnapshot = (linkStyle = 'curve'): Snapshot => {
  const now = Math.floor(Date.now() / 1000);
  const id = (n: number) => `00000000-0000-4000-8000-0000000002${String(n).padStart(2, '0')}`;
  const card = (n: number, title: string, color: string, position: [number, number], next: number[]): Note => ({ id: id(n), text: `# ${title}`, color, modified: now - n * 60, canvas: { position, next: next.map(id), done: false, parent: null } });
  return {
    notes: [
      card(1, '여름 캠페인 기획', 'yellow', [80, 220], [2, 3]),
      card(2, '시안 세 가지', 'blue', [420, 80], [4]),
      card(3, '경쟁사 조사', 'green', [420, 360], []),
      card(4, '최종 시안 고르기', 'purple', [760, 80], []),
      card(5, '주간 회고', 'pink', [760, 420], [6]),
      card(6, '다음 주 계획', 'yellow', [760, 640], [5]),
    ],
    assets: [], revision: 1, savedRevision: 1, preferences: { canvasLinkStyle: linkStyle },
  };
};
/** 성능 측정용 큰 캔버스: 카드 200개(20×10 격자), 간선 400개(오른쪽·아래 이웃). */
const canvasLargeSnapshot = (): Snapshot => {
  const now = Math.floor(Date.now() / 1000);
  const id = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
  const cols = 20; const colors = ['yellow', 'blue', 'green', 'purple', 'pink'];
  const notes: Note[] = Array.from({ length: 200 }, (_, n) => {
    const col = n % cols; const row = Math.floor(n / cols);
    const next = [col < cols - 1 ? n + 1 : (n + cols * 2) % 200, row < 9 ? n + cols : (n + 7) % 200].map(id);
    return { id: id(n), text: `# 카드 ${n + 1}`, color: colors[n % colors.length], modified: now - n, canvas: { position: [80 + col * 340, 80 + row * 220], next, done: false, parent: null } };
  });
  return { notes, assets: [], revision: 1, savedRevision: 1, preferences: {} };
};
export const CanvasLarge: Story = { render: () => previewApp('canvas', undefined, canvasLargeSnapshot(), [1400, 900]) };
export const CanvasBranches: Story = { render: () => previewApp('canvas', undefined, canvasBranchesSnapshot(), [1100, 900]) };
export const CanvasBranchesOrthogonal: Story = { render: () => previewApp('canvas', undefined, canvasBranchesSnapshot('orthogonal'), [1100, 900]) };
export const CanvasBranchesStraight: Story = { render: () => previewApp('canvas', undefined, canvasBranchesSnapshot('straight'), [1100, 900]) };
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
