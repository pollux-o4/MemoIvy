import { useEffect, useMemo, useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react-vite';
import { App } from './App';
import './reference-review.css';
import { createMockBridge, fixtureNoteIds } from '../shared/mock_bridge';
import type { Asset, DesktopBridge, Note, Snapshot } from '../shared/engine_contract';
import canonicalNotes from '../../rooms/desktop-migration/visual-fixture.json';
import imageNotes from '../../docs/assets/gtk-reference/fixtures/image-used/notes.json';
import imageAssets from '../../docs/assets/gtk-reference/fixtures/image-used/assets.json';
import expandedNotes from '../../docs/assets/gtk-reference/fixtures/library-expanded/notes.json';
import expandedProjects from '../../docs/assets/gtk-reference/fixtures/library-expanded/library.json';

type Theme = 'light' | 'dark';
type CaseId = 'library' | 'library-expanded' | 'editor' | 'editor-image' | 'canvas' | 'files-empty-used' | 'files-empty-unused' | 'files-used' | 'files-unused';
type CaseSpec = { title: string; screen: 'library' | 'note' | 'canvas' | 'files'; dataset: 'canonical' | 'library-expanded' | 'image-used'; filename: (theme: Theme) => string; size: [number, number]; filesTab?: 'used' | 'unused'; editorId?: string };
type Capture = { file: string; screen: string; theme: string; width: number; height: number; dataset: string; dataset_sha256: string; renderer?: string; gtk?: string; font?: string; source_git_revision?: string; source_dirty?: boolean; fixture_source?: string; fixture_sha256?: string; capture_command?: string };
type ReferenceManifest = { captures?: Capture[]; captured_unix_seconds?: number; fixture?: { source?: string; sha256?: string }; source?: { git_revision?: string; dirty?: boolean; git_status_short?: string }; runtime?: { gtk?: string; font?: string; display?: string } };

const ids = Object.keys(imageAssets.assets);
const referenceNotes = imageNotes as unknown as Note[];
const assetsById = imageAssets.assets as Record<string, { added: number; last_note?: string; unused_since?: number }>;
const imageAssetRows: Asset[] = ids.map((id, index) => ({
  id,
  name: id === 'dc76b5742600dc4aa297c35649b08ed26ee3b9f7972166b061752681cb134eb8' ? '대시보드 미리보기.png' : '사용하지 않는 이미지.png',
  mime: 'image/png',
  bytes: index === 0 ? 711 : 710,
  added: assetsById[id].added,
  lastNote: assetsById[id].last_note ?? null,
  unusedSince: assetsById[id].unused_since ?? null,
  usedBy: referenceNotes.filter((note) => note.text.includes(`asset:${id}`)).map((note) => note.id),
}));
const basePreferences = { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false };
const canonicalSnapshot: Snapshot = { notes: canonicalNotes as unknown as Note[], assets: [], revision: 1, savedRevision: 1, preferences: basePreferences };
const imageSnapshot: Snapshot = { notes: referenceNotes, assets: imageAssetRows, revision: 1, savedRevision: 1, preferences: basePreferences };
const expandedSnapshot: Snapshot = { ...imageSnapshot, notes: expandedNotes as unknown as Note[], preferences: { ...basePreferences, expandedProjects: expandedProjects as string[] } };

const specs: Record<CaseId, CaseSpec> = {
  library: { title: 'Library · canonical fixture', screen: 'library', dataset: 'canonical', filename: (theme) => `library-${theme}.png`, size: [440, 600] },
  'library-expanded': { title: 'Library · expanded branches', screen: 'library', dataset: 'library-expanded', filename: (theme) => `library-expanded-${theme}.png`, size: [440, 600] },
  editor: { title: 'Editor · canonical fixture', screen: 'note', dataset: 'canonical', filename: (theme) => `editor-${theme}.png`, size: [380, 420], editorId: fixtureNoteIds.ideas },
  'editor-image': { title: 'Editor · attached image', screen: 'note', dataset: 'image-used', filename: (theme) => `editor-image-${theme}.png`, size: [380, 420], editorId: fixtureNoteIds.ideas },
  canvas: { title: 'Canvas · canonical fixture', screen: 'canvas', dataset: 'canonical', filename: (theme) => `canvas-${theme}.png`, size: [1000, 720] },
  'files-empty-used': { title: 'Files · no used files', screen: 'files', dataset: 'canonical', filename: (theme) => `files-empty-used-${theme}.png`, size: [720, 520], filesTab: 'used' },
  'files-empty-unused': { title: 'Files · no unused files', screen: 'files', dataset: 'canonical', filename: (theme) => `files-empty-unused-${theme}.png`, size: [720, 520], filesTab: 'unused' },
  'files-used': { title: 'Files · used and unused', screen: 'files', dataset: 'image-used', filename: (theme) => `files-used-${theme}.png`, size: [720, 545], filesTab: 'used' },
  'files-unused': { title: 'Files · unused assets', screen: 'files', dataset: 'image-used', filename: (theme) => `files-unused-${theme}.png`, size: [720, 545], filesTab: 'unused' },
};

function capturePath(capture: Capture) { return capture.file.split('/').pop() ?? capture.file; }
function toDate(seconds?: number) { return seconds ? new Intl.DateTimeFormat('ko-KR', { dateStyle: 'medium', timeStyle: 'short', timeZone: 'Asia/Seoul' }).format(seconds * 1000) : 'capture time not recorded'; }

function referenceBridge(snapshot: Snapshot): DesktopBridge {
  const bridge = createMockBridge(snapshot);
  const request = bridge.request.bind(bridge);
  return {
    ...bridge,
    async request(op, params) {
      if (op === 'readAsset') {
        const id = String(params?.id ?? '');
        const response = await fetch(`/gtk-reference/fixtures/image-used/assets/${id}.png`);
        if (response.ok) {
          const bytes = new Uint8Array(await response.arrayBuffer());
          let binary = '';
          for (let offset = 0; offset < bytes.length; offset += 0x8000) binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
          return { data: btoa(binary), mime: 'image/png' };
        }
      }
      return request(op, params);
    },
  };
}

function ReferenceReview({ initialCase = 'library' }: { initialCase?: CaseId }) {
  const [caseId, setCaseId] = useState<CaseId>(initialCase);
  const [theme, setTheme] = useState<Theme>('light');
  const [manifest, setManifest] = useState<ReferenceManifest | null>(null);
  const [manifestError, setManifestError] = useState(false);
  useEffect(() => {
    let live = true;
    fetch('/gtk-reference/manifest.json').then((response) => { if (!response.ok) throw new Error('manifest missing'); return response.json() as Promise<ReferenceManifest>; })
      .then((value) => { if (live) setManifest(value); })
      .catch(() => { if (live) setManifestError(true); });
    return () => { live = false; };
  }, []);
  const spec = specs[caseId];
  const expectedFile = spec.filename(theme);
  const record = manifest?.captures?.find((capture) => capturePath(capture) === expectedFile && capture.theme === theme);
  const liveSnapshot = spec.dataset === 'image-used' ? imageSnapshot : spec.dataset === 'library-expanded' ? expandedSnapshot : canonicalSnapshot;
  const bridge: DesktopBridge = useMemo(() => referenceBridge(liveSnapshot), [caseId, liveSnapshot]);
  const frameSize: [number, number] = record ? [record.width, record.height] : spec.size;
  const liveWindow = <div className={`theme-${theme} review-live-window`} style={{ width: frameSize[0], height: frameSize[1] }}><App key={`${caseId}-${theme}`} bridge={bridge} preview={false} initialScreen={spec.screen} initialId={spec.editorId} initialFilesTab={spec.filesTab}/></div>;
  const manifestDate = toDate(manifest?.captured_unix_seconds);
  const metadata = record ? `${record.width} × ${record.height} px · ${theme} · ${manifestDate}` : manifestError ? 'GTK reference manifest is unavailable' : 'Waiting for the finalized native GTK capture';
  const digest = record?.dataset_sha256 ?? record?.fixture_sha256 ?? manifest?.fixture?.sha256;

  return <main className={`reference-review theme-${theme}`}>
    <header className="reference-controls"><div><strong>UI-REF-001</strong><span>GTK 원본 · React 화면 비교</span></div><label>화면<select value={caseId} onChange={(event) => setCaseId(event.target.value as CaseId)}>{Object.entries(specs).map(([id, item]) => <option key={id} value={id}>{item.title}</option>)}</select></label><label>테마<select value={theme} onChange={(event) => setTheme(event.target.value as Theme)}><option value="light">Light</option><option value="dark">Dark</option></select></label><span className="reference-provenance">{record?.renderer ?? manifest?.runtime?.gtk ?? 'Native capture status loads from manifest'}</span></header>
    <div className="reference-metadata"><span>{metadata}</span><span>Dataset: {spec.dataset}</span><code>{digest ?? 'fixture hash pending'}</code></div>
    <div className="reference-pair">
      <section className="reference-panel"><h2>GTK original</h2><div className="reference-frame" style={{ width: frameSize[0], height: frameSize[1] }}>
        {record ? <img src={`/gtk-reference/${expectedFile}`} alt={`Original GTK ${spec.title} ${theme} ${record.width} by ${record.height}`} width={record.width} height={record.height}/> : <div className="reference-placeholder">{manifestError ? 'Capture manifest is unavailable' : `Waiting for ${expectedFile}`}</div>}
      </div><p>{record?.file ?? expectedFile} · {record ? toDate(manifest?.captured_unix_seconds) : 'not yet verified'}</p></section>
      <section className="reference-panel"><h2>React · same fixture</h2><div className="live-frame" style={{ width: frameSize[0], height: frameSize[1] }}>{liveWindow}</div><p>{frameSize[0]} × {frameSize[1]} px · {spec.dataset} · {theme}</p></section>
    </div>
    <footer className="reference-source">Fixture {manifest?.fixture?.source ?? 'rooms/desktop-migration/visual-fixture.json'} · GTK {record?.gtk ?? manifest?.runtime?.gtk ?? 'pending'} · Font {record?.font ?? manifest?.runtime?.font ?? 'pending'} · Source {record?.source_git_revision ?? manifest?.source?.git_revision ?? 'pending'}</footer>
  </main>;
}

const meta: Meta<typeof ReferenceReview> = { title: 'UI review/GTK fidelity', component: ReferenceReview, parameters: { layout: 'fullscreen', controls: { disable: true } } };
export default meta;
type Story = StoryObj<typeof ReferenceReview>;
export const LibraryComparison: Story = { args: { initialCase: 'library' } };
export const EditorComparison: Story = { args: { initialCase: 'editor' } };
export const CanvasComparison: Story = { args: { initialCase: 'canvas' } };
export const FilesComparison: Story = { args: { initialCase: 'files-used' } };
