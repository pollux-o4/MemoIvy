import { useEffect, useEffectEvent, useState } from 'react';
import type { DesktopBridge } from '../shared/engine_contract';
import { useMemoivy } from '../shared/useMemoivy';
import { Icon } from '../shared/Icon';
import { Library } from '../features/library/Library';
import { Editor } from '../features/editor/Editor';
import { Canvas } from '../features/canvas/Canvas';
import { Files } from '../features/attachments/Files';
import '../shared/motion.css';
import { Settings } from '../features/settings/Settings';
import { Help } from '../features/help/Help';
import type { HelpTopic } from '../shared/helpTopic';
import { useAppearance } from '../shared/appearance';
import { useWindowFrame } from '../shared/useWindowFrame';
import { installTooltips } from '../shared/tooltip';

type Screen = 'library' | 'note' | 'canvas' | 'files' | 'settings' | 'help';

export function App({ bridge, preview, initialScreen, initialId, initialFilesTab }: { bridge?: DesktopBridge; preview?: boolean; initialScreen?: Screen; initialId?: string; initialFilesTab?: 'used' | 'unused' }) {
  const store = useMemoivy(bridge);
  const query = new URLSearchParams(window.location.search);
  const [screen, setScreen] = useState<Screen>(() => {
    if (initialScreen) return initialScreen;
    const requested = query.get('window');
    return requested === 'note' || requested === 'canvas' || requested === 'files' || requested === 'settings' || requested === 'help' ? requested : 'library';
  });
  const [activeId, setActiveId] = useState<string | undefined>(() => initialId ?? (initialScreen ? undefined : query.get('id') ?? undefined));
  const [navigationError, setNavigationError] = useState<string | null>(null);
  const [helpTopic, setHelpTopic] = useState<HelpTopic | null>(null);
  useAppearance(store.snapshot?.preferences);
  const frame = useWindowFrame();
  useEffect(() => installTooltips(), []);
  const localPreview = preview ?? (!bridge && !window.memoivy);
  const isProductionFileWithoutBridge = window.location.protocol === 'file:' && !window.memoivy && !bridge;
  const handle = (action: () => void) => (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.isComposing || !(event.ctrlKey || event.metaKey)) return;
    if (event.key.toLowerCase() === 'n') { event.preventDefault(); action(); }
    if (event.key === ',') { event.preventDefault(); void openSettings(); }
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
  const navigate = async (target: 'library' | 'canvas', id?: string) => {
    setNavigationError(null);
    try {
      if (window.memoivy || bridge) {
        await store.bridge.openWindow(target, id);
        return;
      }
      setActiveId(id); setScreen(target);
    } catch {
      setNavigationError('화면을 열지 못했어요. 다시 시도해 주세요.');
    }
  };
  const openHelp = () => setHelpTopic(screen === 'canvas' ? 'canvas' : 'note');
  const openSettings = async () => {
    if (window.memoivy) { await store.bridge.openWindow('settings'); return; }
    setScreen('settings');
  };
  const closeScreen = async () => {
    if (window.memoivy || bridge) { store.bridge.windowAction('close'); return; }
    setScreen('library'); setActiveId(undefined);
  };
  // 전역 단축키는 한 번만 걸고, 누를 때마다 지금 화면의 동작을 읽는다.
  const onGlobalKey = useEffectEvent((event: KeyboardEvent) => {
    if (event.key === 'F1') { event.preventDefault(); if (!helpTopic && screen !== 'help') openHelp(); return; }
    if (!helpTopic) handle(() => { void createNote(false); })(event);
  });
  useEffect(() => {
    const listener = (event: KeyboardEvent) => onGlobalKey(event);
    window.addEventListener('keydown', listener);
    return () => window.removeEventListener('keydown', listener);
  }, []);
  useEffect(() => {
    if (window.memoivy) return;
    const navigate = (event: Event) => {
      const { kind, id } = (event as CustomEvent<{ kind: Screen; id?: string }>).detail;
      if (kind === 'note' || kind === 'canvas' || kind === 'files' || kind === 'library' || kind === 'settings' || kind === 'help') {
        setActiveId(id); setScreen(kind);
      }
    };
    window.addEventListener('memoivy:preview-window', navigate);
    return () => window.removeEventListener('memoivy:preview-window', navigate);
  }, []);

  if (isProductionFileWithoutBridge) return <main className="bridge-failure"><div className="bridge-mark"><Icon name="close"/></div><h1>앱 연결을 시작하지 못했어요</h1><p>MemoIvy 저장 엔진에 연결할 수 없습니다. 입력 데이터는 변경되지 않았습니다.</p><button className="secondary-button" onClick={() => window.location.reload()}>다시 시도</button></main>;

  return <div className={`app-root app-${screen}${frame}`}>
    {localPreview && <div className="preview-banner"><span><i/> 화면 미리보기</span><span>변경 사항은 저장되지 않습니다</span></div>}
    {!store.snapshot && <div className="startup-state"><span className="loading-orbit"/><span>메모를 불러오는 중</span></div>}
    {store.snapshot && screen === 'library' && <Library store={store} onOpenNote={(id) => void openNote(id)} onOpenCanvas={(id) => void navigate('canvas', id)} onCreate={(project) => void createNote(project)} onOpenSettings={() => void openSettings()}/>}
    {screen === 'help' && <Help bridge={store.bridge} onClose={() => void closeScreen()}/>}
    {store.snapshot && screen === 'settings' && <Settings store={store} onClose={() => void closeScreen()}/>}
    {store.snapshot && screen === 'note' && activeId && <Editor store={store} noteId={activeId} onClose={() => void closeScreen()}/>}
    {store.snapshot && screen === 'canvas' && <Canvas store={store} onOpenLibrary={() => void navigate('library')} scope={activeId} onOpenNote={(id) => void openNote(id)} onBack={() => void closeScreen()} onHelp={openHelp} onOpenSettings={() => void openSettings()}/>}
    {store.snapshot && screen === 'files' && <Files store={store} onClose={() => void closeScreen()} onOpenNote={(id) => void openNote(id)} initialTab={initialFilesTab}/>}
    {helpTopic && screen !== 'help' && <Help bridge={store.bridge} initialTopic={helpTopic} onClose={() => setHelpTopic(null)}/>}
    {(navigationError || store.error) && screen !== 'note' && <div className="global-error" role="alert"><Icon name="close" size={15}/><span>{navigationError || store.error}</span><button onClick={() => { if (navigationError) setNavigationError(null); else void store.request('snapshot').catch(() => undefined); }}>다시 확인</button></div>}
  </div>;
}
