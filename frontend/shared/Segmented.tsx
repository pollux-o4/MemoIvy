import type { KeyboardEvent } from 'react';

/** 하나만 고르는 짧은 선택지. Tab으로 들어오고 화살표로 선택을 바꾼다. */
export type SegmentedOption<T extends string> = { value: T; label: string };

export function Segmented<T extends string>({ name, options, value, onChange, className = '' }: {
  name: string; options: readonly SegmentedOption<T>[]; value: T; onChange: (value: T) => void; className?: string;
}) {
  const active = Math.max(0, options.findIndex((option) => option.value === value));
  const move = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const previous = event.key === 'ArrowLeft' || event.key === 'ArrowUp';
    const following = event.key === 'ArrowRight' || event.key === 'ArrowDown';
    if (!previous && !following) return;
    const step = previous ? -1 : 1;
    event.preventDefault();
    const next = (index + step + options.length) % options.length;
    event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus();
    onChange(options[next].value);
  };
  return <div className={`segmented ${className}`.trim()} role="radiogroup" aria-label={name}>{options.map((option, index) =>
    <button key={option.value} type="button" role="radio" aria-checked={value === option.value} tabIndex={index === active ? 0 : -1}
      className={value === option.value ? 'selected' : ''} onClick={() => onChange(option.value)} onKeyDown={(event) => move(event, index)}>{option.label}</button>)}</div>;
}
