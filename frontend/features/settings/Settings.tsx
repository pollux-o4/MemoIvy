import { useState } from 'react';
import { WindowHeader } from '../../shared/WindowHeader';
import { APPEARANCE_KEY, readAppearance, type AppearancePreference, type MotionChoice, type ThemeChoice } from '../../shared/appearance';
import type { MemoStore } from '../../shared/useMemoivy';
import './settings.css';

type Option<T extends string> = { value: T; label: string };
const themeOptions: Option<ThemeChoice>[] = [{ value: 'system', label: '시스템' }, { value: 'light', label: '라이트' }, { value: 'dark', label: '다크' }];
const motionOptions: Option<MotionChoice>[] = [{ value: 'system', label: '시스템' }, { value: 'reduce', label: '켜기' }, { value: 'full', label: '끄기' }];

export function Settings({ store, onClose }: { store: MemoStore; onClose: () => void }) {
  const saved = readAppearance(store.snapshot?.preferences);
  const [draft, setDraft] = useState<AppearancePreference | null>(null);
  const appearance = draft ?? saved;
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
      <Section title="기록">
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

function Segmented<T extends string>({ name, options, value, onChange }: { name: string; options: Option<T>[]; value: T; onChange: (value: T) => void }) {
  return <div className="segmented" role="radiogroup" aria-label={name}>{options.map((option) => <button key={option.value} role="radio" aria-checked={value === option.value} className={value === option.value ? 'selected' : ''} onClick={() => onChange(option.value)}>{option.label}</button>)}</div>;
}
