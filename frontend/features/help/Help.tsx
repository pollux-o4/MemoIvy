import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { Icon } from '../../shared/Icon';
import type { DesktopBridge } from '../../shared/engine_contract';
import { WindowHeader } from '../../shared/WindowHeader';
import { HELP_TOPIC_KEY, type HelpTopic, readHelpTopic, rememberHelpTopic } from '../../shared/helpTopic';
import { HelpContent } from './HelpContent';
import './help.css';

/**
 * 단축키와 서식: 위에서 아래로 좁혀 읽는다. 주제(메모·캔버스) → 기본 동작 4개 → 전체 목록(묶음별로 펼치기).
 * 처음 주제는 도움말을 연 창의 맥락을 따른다.
 */
export function Help({ bridge, initialTopic, onClose }: { bridge: DesktopBridge; initialTopic?: HelpTopic; onClose: () => void }) {
  const [topic, setTopic] = useState<HelpTopic>(() => initialTopic ?? readHelpTopic());
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useLayoutEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
  }, []);
  useEffect(() => {
    if (initialTopic) return;
    const follow = (event: StorageEvent) => { if (event.key === HELP_TOPIC_KEY) setTopic(readHelpTopic()); };
    window.addEventListener('storage', follow);
    return () => window.removeEventListener('storage', follow);
  }, [initialTopic]);
  const choose = (next: HelpTopic) => { setTopic(next); if (!initialTopic) rememberHelpTopic(next); };
  const content = <HelpContent topic={topic} onTopicChange={choose}/>;
  if (!initialTopic) return <div className="help-screen">
    <WindowHeader className="window-header help-bar" title="단축키와 서식" bridge={bridge} onClose={onClose} controls={[]}/>
    {content}
  </div>;
  const close = () => { dialog.current?.close(); onClose(); };
  return <dialog ref={dialog} className="help-dialog" aria-labelledby={titleId} onCancel={(event) => { event.preventDefault(); close(); }}>
    <div className="help-screen">
      <header className="help-dialog-header">
        <h2 id={titleId}>단축키와 서식</h2>
        <button type="button" className="icon-button" aria-label="도움말 닫기" onClick={close}><Icon name="close" size={16}/></button>
      </header>
      {content}
    </div>
  </dialog>;
}
