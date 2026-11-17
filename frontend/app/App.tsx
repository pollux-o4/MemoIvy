import { useEffect, useState } from 'react';
import type { DesktopBridge } from '../shared/engine_contract';
import { useMemoivy } from '../shared/useMemoivy';
import { Icon } from '../shared/Icon';
import { Library } from '../features/library/Library';
import { Editor } from '../features/editor/Editor';
import { Canvas } from '../features/canvas/Canvas';
import { Files } from '../features/attachments/Files';

type Screen = 'library' | 'note' | 'canvas' | 'files';

export function App({ bridge, preview, initialScreen, initialId, initialFilesTab }: { bridge?: DesktopBridge; preview?: boolean; initialScreen?: 'library' | 'note' | 'canvas' | 'files'; initialId?: string; initialFilesTab?: 'used' | 'unused' }) {
  const store = useMemoivy(bridge);
  const query = new URLSearchParams(window.location.search);
  const [screen, setScreen] = useState<Screen>(() => {
    if (initialScreen) return initialScreen;
    const requested = query.get('window');
    return requested === 'note' || requested === 'canvas' || requested === 'files' ? requested : 'library';
  });
  const [activeId, setActiveId] = useState<string | undefined>(() => initialId ?? (initialScreen ? undefined : query.get('id') ?? undefined));
  const [showHelp, setShowHelp] = useState(false);
  const localPreview = preview ?? (!bridge && !window.memoivy);
  const isProductionFileWithoutBridge = window.location.protocol === 'file:' && !window.memoivy && !bridge;
  const handle = (action: () => void) => (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.isComposing || !(event.ctrlKey || event.metaKey)) return;
    if (event.key.toLowerCase() === 'n') { event.preventDefault(); action(); }
    if (event.key.toLowerCase() === 'f' && screen === 'library') { event.preventDefault(); document.getElementById('library-search')?.focus(); }
  };
  const createNote = async (project: boolean) => {
    try { const note = await store.createNote({ project, text: project ? '# 새 프로젝트\n' : '' }); if (note) await openNote(note.id); }
    catch { /* the store exposes the error and preserves the current screen */ }
  };
  const openNote = async (id: string) => {
    if (window.memoivy) { await store.bridge.openNote(id); return; }
    setActiveId(id); setScreen('note');
  };
  const openCanvas = async (id?: string) => {
    if (window.memoivy) { await store.bridge.openWindow('canvas', id); return; }
    setActiveId(id); setScreen('canvas');
  };
  const closeScreen = async () => {
    if (window.memoivy || bridge) { store.bridge.windowAction('close'); return; }
    setScreen('library'); setActiveId(undefined);
  };
  useEffect(() => {
    const create = () => { void createNote(false); };
    const listener = handle(create);
    window.addEventListener('keydown', listener);
    const help = (event: KeyboardEvent) => { if (event.key === 'F1') { event.preventDefault(); setShowHelp(true); } };
    window.addEventListener('keydown', help);
    return () => { window.removeEventListener('keydown', listener); window.removeEventListener('keydown', help); };
  }, [screen]);
  useEffect(() => {
    if (window.memoivy) return;
    const navigate = (event: Event) => {
      const { kind, id } = (event as CustomEvent<{ kind: Screen; id?: string }>).detail;
      if (kind === 'note' || kind === 'canvas' || kind === 'files' || kind === 'library') {
        setActiveId(id); setScreen(kind);
      }
    };
    window.addEventListener('memoivy:preview-window', navigate);
    return () => window.removeEventListener('memoivy:preview-window', navigate);
  }, []);

  if (isProductionFileWithoutBridge) return <main className="bridge-failure"><div className="bridge-mark"><Icon name="close"/></div><h1>앱 연결을 시작하지 못했어요</h1><p>MemoIvy 저장 엔진에 연결할 수 없습니다. 입력 데이터는 변경되지 않았습니다.</p><button className="secondary-button" onClick={() => window.location.reload()}>다시 시도</button></main>;

  return <div className={`app-root app-${screen}`}>
    {localPreview && <div className="preview-banner"><span><i/> 화면 미리보기</span><span>변경 사항은 저장되지 않습니다</span></div>}
    {!store.snapshot && <div className="startup-state"><span className="loading-orbit"/><span>메모를 불러오는 중</span></div>}
    {store.snapshot && screen === 'library' && <Library store={store} onOpenNote={(id) => void openNote(id)} onOpenCanvas={(id) => void openCanvas(id)} onCreate={(project) => void createNote(project)}/>}
    {store.snapshot && screen === 'note' && activeId && <Editor store={store} noteId={activeId} onClose={() => void closeScreen()}/>}
    {store.snapshot && screen === 'canvas' && <Canvas store={store} scope={activeId} onOpenNote={(id) => void openNote(id)} onBack={() => void closeScreen()}/>}
    {store.snapshot && screen === 'files' && <Files store={store} onClose={() => void closeScreen()} onOpenNote={(id) => void openNote(id)} initialTab={initialFilesTab}/>}
    {store.error && screen !== 'note' && <div className="global-error" role="alert"><Icon name="close" size={15}/><span>{store.error}</span><button onClick={() => void store.request('snapshot').catch(() => undefined)}>다시 확인</button></div>}
    {showHelp && <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowHelp(false); }}><section className="shortcut-dialog" role="dialog" aria-modal="true" aria-labelledby="shortcut-title"><button className="icon-button shortcut-close" autoFocus aria-label="닫기" onClick={() => setShowHelp(false)}><Icon name="close"/></button><p className="eyebrow">KEYBOARD SHORTCUTS</p><h2 id="shortcut-title">빠른 단축키</h2><dl><div><dt>새 메모</dt><dd><kbd>Ctrl</kbd><kbd>N</kbd></dd></div><div><dt>목록 검색</dt><dd><kbd>Ctrl</kbd><kbd>F</kbd></dd></div><div><dt>현재 창 닫기</dt><dd><kbd>Ctrl</kbd><kbd>W</kbd></dd></div><div><dt>Markdown 원문 전환</dt><dd><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>M</kbd></dd></div><div><dt>할 일 완료</dt><dd><kbd>Ctrl</kbd><kbd>Enter</kbd></dd></div><div><dt>단축키 안내</dt><dd><kbd>F1</kbd></dd></div></dl><button className="secondary-button shortcut-done" onClick={() => setShowHelp(false)}>확인</button></section></div>}
  </div>;
}
