import { Icon } from './Icon';

export function SettingsButton({ onClick }: { onClick: () => void }) {
  return <button type="button" className="icon-button" title="설정" aria-label="설정" onClick={onClick}><Icon name="settings"/></button>;
}
