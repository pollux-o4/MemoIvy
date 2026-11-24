import { useEffect, useId, useRef } from 'react';
import { useRovingFocus } from './useRovingFocus';

/**
 * 공통 확인 대화상자: 폭 340, 왼쪽 정렬 질문·대상·설명, 같은 폭의 [취소][확정],
 * 기본 포커스는 취소, Esc·바깥 클릭은 취소.
 */
export function ConfirmDialog({ question, subject, detail, acceptLabel, onCancel, onAccept }: {
  question: string; subject?: string; detail: string; acceptLabel: string; onCancel: () => void; onAccept: () => void;
}) {
  const titleId = useId();
  const detailId = useId();
  const cancel = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLElement>(null);
  const buttons = useRovingFocus(dialog, { orientation: 'horizontal', loop: true });
  useEffect(() => {
    cancel.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); onCancel(); }
      if (event.key !== 'Tab' || event.altKey || event.ctrlKey || event.metaKey) return;
      const items = buttons();
      if (!items.length) return;
      const active = document.activeElement;
      const index = active instanceof HTMLButtonElement ? items.indexOf(active) : -1;
      const target = index < 0 ? (event.shiftKey ? items.length - 1 : 0)
        : (index + (event.shiftKey ? -1 : 1) + items.length) % items.length;
      event.preventDefault();
      event.stopPropagation();
      items[target].focus();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [onCancel, buttons]);
  // biome-ignore lint/a11y/noStaticElementInteractions: 배경 클릭 닫기는 포인터 편의다. 키보드는 Esc·취소 버튼으로 닫는다.
  return <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel(); }}>
    <section ref={dialog} className="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId} aria-describedby={detailId}>
      <h2 id={titleId} className="confirm-title">{question}</h2>
      {subject && <p className="confirm-subject">{subject}</p>}
      <p id={detailId} className="confirm-detail">{detail}</p>
      <div className="confirm-actions">
        <button ref={cancel} type="button" className="confirm-cancel" onClick={onCancel}>취소</button>
        <button type="button" className="confirm-accept" onClick={onAccept}>{acceptLabel}</button>
      </div>
    </section>
  </div>;
}
