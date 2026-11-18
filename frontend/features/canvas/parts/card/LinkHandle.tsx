import { Icon } from '../../../../shared/Icon';

/** 다음 메모 연결 손잡이: 끌어서 새 연결을 시작하고, 현재 연결 수를 보인다. */
export function LinkHandle({ title, count, onConnectStart }: { title: string; count: number; onConnectStart: () => void }) {
  return <footer>
    <button title="끌어서 다음 메모에 연결" aria-label={`${title} 다음 메모 연결`} onPointerDown={(event) => { event.stopPropagation(); onConnectStart(); }}>
      <Icon name="link" size={14}/>
    </button>
    <span>{count}</span>
  </footer>;
}
