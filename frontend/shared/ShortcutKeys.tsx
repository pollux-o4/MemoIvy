/** 같은 동작의 키 여럿을 "또는"으로 이어 키캡으로 보인다. 모양은 controls.css `.shortcut-keys`. */
export function ShortcutKeys({ keys, className = '' }: { keys: readonly string[]; className?: string }) {
  return <span className={`shortcut-keys ${className}`.trim()}>{keys.map((key, index) => <span key={key}>{index > 0 && <i>또는</i>}<kbd className="shortcut-key">{key}</kbd></span>)}</span>;
}
