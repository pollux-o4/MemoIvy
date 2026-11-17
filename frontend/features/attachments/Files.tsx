import { useEffect, useMemo, useRef, useState } from 'react';
import type { Asset } from '../../shared/engine_contract';
import { Icon } from '../../shared/Icon';
import { titleOf } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';
import './files.css';

function reference(asset: Asset) { return `![${asset.name.replaceAll(']', '').replaceAll('\n', '')}](asset:${asset.id})`; }
function readableBytes(bytes: number) { return bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${Math.round(bytes / 1024)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`; }
function daysAgo(at: number | undefined) {
  if (!at) return '오늘';
  const days = Math.floor((Date.now() / 1000 - at) / 86_400);
  return days <= 0 ? '오늘' : days === 1 ? '어제' : `${days}일 전`;
}

export function Files({ store, onClose, onOpenNote, initialTab = 'used' }: { store: MemoStore; onClose: () => void; onOpenNote: (id: string) => void; initialTab?: 'used' | 'unused' }) {
  const [tab, setTab] = useState<'used' | 'unused'>(initialTab);
  const [selected, setSelected] = useState<string | null>(null);
  const [urls, setUrls] = useState<Record<string, string>>({});
  const [confirm, setConfirm] = useState<'one' | 'all' | null>(null);
  const [viewing, setViewing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [autoPrune, setAutoPrune] = useState(Boolean(store.snapshot?.preferences?.autoPrune));
  const objectUrls = useRef<string[]>([]);
  const confirmCancel = useRef<HTMLButtonElement>(null);
  const confirmTrigger = useRef<HTMLElement | null>(null);
  const usedIds = useMemo(() => new Set(store.notes.flatMap((note) => [...note.text.matchAll(/asset:([a-f0-9]{64})/gi)].map((match) => match[1]))), [store.notes]);
  const assets = store.assets.map((asset) => ({ ...asset, usedBy: store.notes.filter((note) => note.text.includes(`asset:${asset.id}`)).map((note) => note.id) }));
  const used = assets.filter((asset) => usedIds.has(asset.id));
  const unused = assets.filter((asset) => !usedIds.has(asset.id));
  const visible = tab === 'used' ? used : unused;
  const active = assets.find((asset) => asset.id === selected) ?? visible[0];
  const lastNote = active?.usedBy[0] ?? (active?.lastNote ? store.notes.find((note) => note.id === active.lastNote)?.id : undefined);

  useEffect(() => {
    let live = true;
    store.assets.forEach((asset) => {
      if (urls[asset.id]) return;
      void store.request('readAsset', { id: asset.id }).then((result) => {
        if (!live || !result.data) return;
        const binary = atob(result.data);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
        const url = URL.createObjectURL(new Blob([bytes], { type: result.mime ?? asset.mime }));
        objectUrls.current.push(url);
        setUrls((current) => ({ ...current, [asset.id]: url }));
      }).catch(() => undefined);
    });
    return () => { live = false; };
  }, [store.assets]);
  useEffect(() => () => objectUrls.current.forEach((url) => URL.revokeObjectURL(url)), []);
  useEffect(() => setAutoPrune(Boolean(store.snapshot?.preferences?.autoPrune)), [store.snapshot?.preferences?.autoPrune]);
  useEffect(() => {
    if (viewing) {
      const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') setViewing(false); };
      window.addEventListener('keydown', escape);
      return () => window.removeEventListener('keydown', escape);
    }
  }, [viewing]);
  useEffect(() => {
    if (!confirm) { confirmTrigger.current?.focus(); return; }
    confirmCancel.current?.focus();
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); setConfirm(null); } };
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [confirm]);

  const removeOne = async (asset: Asset) => {
    try { await store.request('deleteAsset', { id: asset.id }); setSelected(null); setMessage('파일을 삭제했어요.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : String(error)); }
    setConfirm(null);
  };
  const removeAll = async () => {
    for (const asset of unused) {
      try { await store.request('deleteAsset', { id: asset.id }); }
      catch (error) { setMessage(error instanceof Error ? error.message : String(error)); setConfirm(null); return; }
    }
    setMessage('사용하지 않는 파일을 모두 삭제했어요.');
    setConfirm(null);
    setSelected(null);
  };
  const setAutoPruneValue = async (value: boolean) => {
    setAutoPrune(value);
    try { await store.request('preference', { key: 'autoPrune', value }); }
    catch (error) { setAutoPrune(!value); setMessage(error instanceof Error ? error.message : String(error)); }
  };
  const copyReference = async (asset: Asset) => { await navigator.clipboard?.writeText(reference(asset)); setMessage('참조를 복사했어요.'); };

  return <div className="files-screen">
    <header className="files-header">
      <div className="window-controls"><button aria-label="닫기" onClick={onClose}><Icon name="close"/></button><button aria-label="최소화" onClick={() => store.bridge.windowAction('minimize')}><Icon name="minimize"/></button><button aria-label="최대화" onClick={() => store.bridge.windowAction('maximize')}><Icon name="maximize"/></button></div>
      <h1 className="files-title">파일</h1><span/>
    </header>
    <div className="files-toolbar">
      <div className="view-tabs"><button className={tab === 'used' ? 'selected' : ''} onClick={() => { setSelected(null); setTab('used'); }}>사용 중 <b>{used.length}</b></button><button className={tab === 'unused' ? 'selected' : ''} onClick={() => { setSelected(null); setTab('unused'); }}>사용 안 함 <b>{unused.length}</b></button></div>
      {tab === 'unused' && <><label className="toggle-row"><input type="checkbox" checked={autoPrune} onChange={(event) => void setAutoPruneValue(event.target.checked)}/><span className="toggle-slider"/><span>30일 지난 항목 자동 정리</span></label>{unused.length > 0 && <button className="danger-quiet" onClick={(event) => { confirmTrigger.current = event.currentTarget; setConfirm('all'); }}>사용 안 함 모두 삭제…</button>}</>}
    </div>
    <div className="files-layout">
      <section className="asset-grid" aria-label={tab === 'used' ? '사용 중인 파일' : '사용하지 않는 파일'}>
        {visible.map((asset) => <button key={asset.id} className={`asset-tile${asset.id === active?.id ? ' selected' : ''}`} onClick={() => setSelected(asset.id)} draggable onDragStart={(event) => event.dataTransfer.setData('text/plain', reference(asset))} title={`${asset.name} · ${readableBytes(asset.bytes)}`} aria-label={asset.name}>
          <span className="asset-thumb">{urls[asset.id] ? <img src={urls[asset.id]} alt=""/> : <Icon name="file" size={26}/>}</span>
        </button>)}
        {visible.length === 0 && <div className="files-empty">{tab === 'used' ? '메모에 스크린샷을 붙여 넣거나 이미지 파일을 끌어 놓으면 여기에 모여요.' : '메모에서 뺀 이미지가 여기에 남아요. 다시 쓰려면 메모로 끌어 놓으세요.'}</div>}
      </section>
      {active && <aside className="asset-detail">
        <div className="asset-preview">{urls[active.id] ? <img src={urls[active.id]} alt={active.name}/> : <div className="asset-placeholder"><Icon name="file" size={38}/><span>미리보기를 불러올 수 없습니다</span></div>}</div>
        <div className="asset-info">
          <p className="asset-meta">{readableBytes(active.bytes)} · {daysAgo(active.added)} 추가</p>
          <h2>{active.usedBy.length ? '사용 중인 메모' : `마지막 사용 · ${daysAgo(active.unusedSince ?? undefined)}`}</h2>
          <div className="asset-notes">{active.usedBy.length ? active.usedBy.map((id) => {
            const note = store.notes.find((item) => item.id === id);
            return note && <button key={id} onClick={() => onOpenNote(id)}>{titleOf(note)}</button>;
          }) : lastNote ? <button onClick={() => onOpenNote(lastNote)}>{store.notes.find((note) => note.id === lastNote) ? titleOf(store.notes.find((note) => note.id === lastNote)!) : '삭제된 메모'}</button> : <span>쓰인 적 없음</span>}</div>
          <div className="asset-actions">
            <button onClick={() => setViewing(true)}><Icon name="expand" size={16}/> 크게 보기</button>
            <button onClick={() => void copyReference(active)}><Icon name="copy" size={16}/> 참조 복사</button>
            {!active.usedBy.length && <button className="asset-delete" onClick={(event) => { confirmTrigger.current = event.currentTarget; setConfirm('one'); }}><Icon name="trash" size={16}/> 삭제…</button>}
          </div>
        </div>
      </aside>}
    </div>
    {message && <div className="toast" role="status">{message}<button aria-label="닫기" onClick={() => setMessage(null)}><Icon name="close" size={15}/></button></div>}
    {viewing && active && urls[active.id] && <div className="asset-viewer" role="dialog" aria-label="이미지 크게 보기" aria-modal="true" onClick={() => setViewing(false)}><button aria-label="닫기" onClick={() => setViewing(false)}><Icon name="close"/></button><img src={urls[active.id]} alt={active.name}/></div>}
    {confirm && <div className="modal-backdrop" role="presentation"><section className="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title"><div className="confirm-mark"><Icon name="file"/></div><h2 id="confirm-title">{confirm === 'one' ? '파일을 영구 삭제할까요?' : '사용하지 않는 파일을 모두 삭제할까요?'}</h2><p>삭제한 파일은 복구할 수 없습니다.</p><div><button ref={confirmCancel} className="secondary-button" onClick={() => setConfirm(null)}>취소</button><button className="danger-button" onClick={() => confirm === 'one' && active ? void removeOne(active) : void removeAll()}>삭제</button></div></section></div>}
  </div>;
}
