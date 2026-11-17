import type { Meta, StoryObj } from '@storybook/react-vite';
import { App } from './App';
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
function previewApp(screen: 'library' | 'note' | 'canvas' | 'files', id?: string, snapshot?: Snapshot, customSize?: number[]) {
  const size = customSize ?? (screen === 'library' ? [440, 600] : screen === 'note' ? [380, 420] : screen === 'canvas' ? [1000, 720] : [720, 520]);
  return <div className="theme-light" style={{ width: size[0], height: size[1], margin: '0 auto', background: 'var(--memoivy-bg)' }}><App bridge={storyBridge(snapshot)} preview={false} initialScreen={screen} initialId={id}/></div>;
}

const meta: Meta<typeof App> = { title: 'MemoIvy/Window states', component: App, parameters: { layout: 'fullscreen' } };
export default meta;
type Story = StoryObj<typeof App>;

export const Library: Story = { render: () => previewApp('library') };
export const Editor: Story = { render: () => previewApp('note', fixtureNoteIds.project) };
export const Canvas: Story = { render: () => previewApp('canvas') };
export const Files: Story = { render: () => previewApp('files', undefined, makeSnapshot()) };
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
