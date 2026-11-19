import { useState } from 'react';
import { Segmented, type SegmentedOption } from '../../shared/Segmented';
import { WindowHeader } from '../../shared/WindowHeader';
import { APPEARANCE_KEY, readAppearance, type AppearancePreference, type MotionChoice, type ThemeChoice } from '../../shared/appearance';
import { LINK_STYLE_KEY, LINK_STYLE_OPTIONS, readLinkStyle } from '../../shared/link_style';
import type { MemoStore } from '../../shared/useMemoivy';
import './settings.css';

const themeOptions: SegmentedOption<ThemeChoice>[] = [{ value: 'system', label: '시스템' }, { value: 'light', label: '라이트' }, { value: 'dark', label: '다크' }];
const motionOptions: SegmentedOption<MotionChoice>[] = [{ value: 'system', label: '시스템' }, { value: 'reduce', label: '켜기' }, { value: 'full', label: '끄기' }];

export function Settings({ store, onClose }: { store: MemoStore; onClose: () => void }) {
  const saved = readAppearance(store.snapshot?.preferences);
  const [draft, setDraft] = useState<AppearancePreference | null>(null);
  const appearance = draft ?? saved;
  const [importState, setImportState] = useState<'idle' | 'pending' | 'done' | 'failed'>('idle');
  const importLegacy = async () => {
    setImportState('pending');
    try {
      const result = await store.bridge.importLegacy();
      setImportState(result ? 'done' : 'idle');
    } catch {
      setImportState('failed');
    }
  };
  const linkStyle = readLinkStyle(store.snapshot?.preferences);
  const update = (patch: Partial<AppearancePreference>) => {
    const next = { ...appearance, ...patch };
    setDraft(next);
    void store.request('preference', { key: APPEARANCE_KEY, value: next }).then(() => setDraft(null)).catch(() => setDraft(null));
  };

  return <div className="settings-screen">
    <WindowHeader title="설정" bridge={store.bridge} onClose={onClose} controls={['minimize']}/>
    <main className="settings-main">
      <Section title="화면">
        <Row label="테마" hint="시스템은 운영체제의 라이트·다크 설정을 따릅니다."><Segmented name="테마" options={themeOptions} value={appearance.theme} onChange={(theme) => update({ theme })}/></Row>
        <Row label="모션 줄이기" hint="켜면 확대·전환 애니메이션 대신 결과만 바로 보여 줍니다."><Segmented name="모션 줄이기" options={motionOptions} value={appearance.motion} onChange={(motion) => update({ motion })}/></Row>
      </Section>
      <Section title="캔버스">
        <Row label="연결선 모양" hint="연결점은 카드 위치에 맞춰 자동으로 정해집니다."><Segmented name="연결선 모양" options={LINK_STYLE_OPTIONS} value={linkStyle} onChange={(value) => void store.request('preference', { key: LINK_STYLE_KEY, value }).catch(() => undefined)}/></Row>
      </Section>
      <Section title="기록">
        <Row label="이전 메모" hint="빈 저장소에 notes.json과 같은 폴더의 첨부 파일을 가져옵니다.">
          <button className="secondary-button" disabled={!window.memoivy || importState === 'pending'} onClick={() => void importLegacy()}>가져오기</button>
        </Row>
        <div className="settings-import-status" role="status">{{
          idle: '',
          pending: '가져오는 중…',
          done: '이전 메모를 가져왔습니다.',
          failed: '가져오지 못했습니다. 저장소가 비어 있고 파일을 읽을 수 있는지 확인해 주세요.',
        }[importState]}</div>
        <Row label="백업" hint="메모와 첨부 파일을 한 파일로 저장합니다."><button className="secondary-button" disabled>준비 중</button></Row>
        <Row label="내보내기" hint="Markdown 파일로 내보냅니다."><button className="secondary-button" disabled>준비 중</button></Row>
      </Section>
      <Section title="정보">
        <Row label="MemoIvy" hint="데스크톱 메모"><span className="settings-value">0.1.2</span></Row>
      </Section>
    </main>
  </div>;
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="settings-section" aria-label={title}><h2>{title}</h2><div className="settings-group">{children}</div></section>;
}

function Row({ label, hint, children }: { label: string; hint: string; children: React.ReactNode }) {
  return <div className="settings-row"><div><b>{label}</b><small>{hint}</small></div>{children}</div>;
}
