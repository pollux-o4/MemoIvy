import { useEffect, useId, useRef } from 'react';

/**
 * GTK shared/menus/dialogs.rs: 폭 340, 왼쪽 정렬 질문·대상·설명, 같은 폭의 [취소][확정],
 * 기본 포커스는 취소, Esc·바깥 클릭은 취소.
 */
export function ConfirmDialog({ question, subject, detail, acceptLabel, onCancel, onAccept }: {
  question: string; subject?: string; detail: string; acceptLabel: string; onCancel: () => void; onAccept: () => void;
}) {
  const titleId = useId();
  const detailId = useId();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    cancel.current?.focus();
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); onCancel(); } };
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [onCancel]);
  return <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel(); }}>
    <section className="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId} aria-describedby={detailId}>
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
