import { Icon } from './Icon';

export function Disclosure({ expanded, onToggle, targetId, label }: { expanded: boolean; onToggle: () => void; targetId: string; label: string }) {
  return <button className={`disclosure${expanded ? ' is-open' : ''}`} type="button" aria-label={`${label} ${expanded ? '접기' : '펼치기'}`} aria-expanded={expanded} aria-controls={targetId} title={`${label} ${expanded ? '접기' : '펼치기'}`} onClick={onToggle}><Icon name="chevron" size={18}/></button>;
}
