import type { PointerEvent } from 'react';
import { Icon } from '../../../../shared/Icon';

/**
 * 마인드맵 가지 손잡이(카드 오른쪽 가운데). 끌어서 놓거나, 눌러서 이을 곳을 고른다: 카드면 연결, 빈 곳이면 그 자리에 새 가지.
 * 키보드(Enter·Space)는 고를 포인터가 없으니 바로 하위 가지를 만든다.
 * 카드에 올리거나 고르면 보인다(평소 화면은 조용하게).
 */
export function BranchHandle({ title, onConnectStart, onAdd }: { title: string; onConnectStart: (event: PointerEvent) => void; onAdd: () => void }) {
  return <button className="card-branch" title="눌러서 이을 곳 고르기 · 끌어서 바로 잇기" aria-label={`${title}에서 하위 가지 만들기`} aria-keyshortcuts="Tab"
    onPointerDown={(event) => { if (event.button !== 0) return; event.stopPropagation(); onConnectStart(event); }}
    // 마우스 누르기는 보드가 포인터를 잡아 처리하고, 여기 click은 키보드(Enter·Space)에서만 온다.
    onClick={(event) => { event.stopPropagation(); if (event.detail === 0) onAdd(); }}>
    <Icon name="plus" size={14}/>
  </button>;
}
