import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import type { Asset } from '../../shared/engine_contract';
import { WindowHeader } from '../../shared/WindowHeader';
import { Icon } from '../../shared/Icon';
import { titleOf } from '../../shared/model';
import type { MemoStore } from '../../shared/useMemoivy';
import { ImageViewer } from '../../shared/ImageViewer';
import { ConfirmDialog } from '../../shared/ConfirmDialog';
import { daysAgo, readableBytes, reference, usersByAsset } from './files_model';
import './files.css';

export function Files({ store, onClose, onOpenNote, initialTab = 'used' }: { store: MemoStore; onClose: () => void; onOpenNote: (id: string) => void; initialTab?: 'used' | 'unused' }) {
  const [tab, setTab] = useState<'used' | 'unused'>(initialTab);
  const [selected, setSelected] = useState<string | null>(null);
  const [urls, setUrls] = useState<Record<string, string>>({});
  const [confirm, setConfirm] = useState<'one' | 'all' | null>(null);
  const [viewing, setViewing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [autoPrune, setAutoPrune] = useState(Boolean(store.snapshot?.preferences?.autoPrune));
  const objectUrls = useRef<string[]>([]);
  const gridRef = useRef<HTMLElement>(null);
  const confirmTrigger = useRef<HTMLElement | null>(null);
  const assets = useMemo(() => {
    const users = usersByAsset(store.notes);
    return store.assets.map((asset) => ({ ...asset, usedBy: users.get(asset.id) ?? [] }));
  }, [store.assets, store.notes]);
  const used = assets.filter((asset) => asset.usedBy.length > 0);
  const unused = assets.filter((asset) => asset.usedBy.length === 0);
  const visible = tab === 'used' ? used : unused;
  const active = assets.find((asset) => asset.id === selected) ?? visible[0];
  const lastNote = active?.usedBy[0] ?? active?.lastNote ?? undefined;
  const lastNoteItem = lastNote ? store.notes.find((note) => note.id === lastNote) : undefined;

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
  useEffect(() => { if (!confirm) confirmTrigger.current?.focus(); }, [confirm]);

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
  const openViewer = () => setViewing(true);
  const moveGridFocus = (event: ReactKeyboardEvent<HTMLButtonElement>, index: number) => {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const columns = Math.max(1, Math.floor(((gridRef.current?.clientWidth ?? 128) - 24) / 128));
    const delta = event.key === 'ArrowLeft' ? -1 : event.key === 'ArrowRight' ? 1 : event.key === 'ArrowUp' ? -columns : event.key === 'ArrowDown' ? columns : 0;
    const target = event.key === 'Home' ? 0 : event.key === 'End' ? visible.length - 1 : index + delta;
    if (target < 0 || target >= visible.length || target === index) return;
    setSelected(visible[target].id);
    gridRef.current?.querySelectorAll<HTMLButtonElement>('.asset-tile')[target]?.focus();
  };
  const copyReference = async (asset: Asset) => {
    try { await navigator.clipboard?.writeText(reference(asset)); setMessage('복사했어요. 메모에 붙여 넣으세요.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : String(error)); }
  };

  return <div className="files-screen">
    <WindowHeader className="files-header" title={<h1 className="files-title">파일</h1>} bridge={store.bridge} onClose={onClose}/>
    <div className="files-toolbar">
      <div className="segmented view-tabs" aria-label="파일 보기"><button type="button" aria-pressed={tab === 'used'} className={tab === 'used' ? 'selected' : ''} onClick={() => { setSelected(null); setTab('used'); }}>사용 중 <b>{used.length}</b></button><button type="button" aria-pressed={tab === 'unused'} className={tab === 'unused' ? 'selected' : ''} onClick={() => { setSelected(null); setTab('unused'); }}>사용 안 함 <b>{unused.length}</b></button></div>
      {tab === 'unused' && <><label className="toggle-row" title="사용 안 함이 된 지 30일 지난 파일을 앱을 열 때 지워요"><input type="checkbox" checked={autoPrune} onChange={(event) => void setAutoPruneValue(event.target.checked)}/><span className="toggle-slider"/><span>30일 지난 항목 자동 정리</span></label>{unused.length > 0 && <button className="danger-quiet" onClick={(event) => { confirmTrigger.current = event.currentTarget; setConfirm('all'); }}>사용 안 함 모두 삭제…</button>}</>}
    </div>
    <div className="files-layout">
      <section ref={gridRef} className="asset-grid" aria-label={tab === 'used' ? '사용 중인 파일' : '사용하지 않는 파일'}>
        {visible.map((asset, index) => <button key={asset.id} type="button" tabIndex={asset.id === active?.id ? 0 : -1} className={`asset-tile${asset.id === active?.id ? ' selected' : ''}`} onClick={() => setSelected(asset.id)} onKeyDown={(event) => moveGridFocus(event, index)} draggable onDragStart={(event) => event.dataTransfer.setData('text/plain', reference(asset))} title={`${asset.name} · ${readableBytes(asset.bytes)}`} aria-label={asset.name}>
          <span className="asset-thumb">{urls[asset.id] ? <img src={urls[asset.id]} alt=""/> : <Icon name="file" size={26}/>}</span>
        </button>)}
        {visible.length === 0 && <div className="files-empty">{tab === 'used' ? '메모에 스크린샷을 붙여 넣거나 이미지 파일을 끌어 놓으면 여기에 모여요.' : '메모에서 뺀 이미지가 여기에 남아요. 다시 쓰려면 메모로 끌어 놓으세요.'}</div>}
      </section>
      {active && <aside key={active.id} className="asset-detail">
        <div className="asset-preview">{urls[active.id] ? <img src={urls[active.id]} alt={active.name}/> : <div className="asset-placeholder"><Icon name="file" size={38}/><span>미리보기를 불러올 수 없습니다</span></div>}</div>
        <div className="asset-info">
          <p className="asset-meta">{readableBytes(active.bytes)} · {daysAgo(active.added)} 추가</p>
          <h2>{active.usedBy.length ? '사용 중인 메모' : active.unusedSince ? `마지막 사용 · ${daysAgo(active.unusedSince)}` : '마지막 사용'}</h2>
          <div className="asset-notes">{active.usedBy.length ? active.usedBy.map((id) => {
            const note = store.notes.find((item) => item.id === id);
            return note && <button key={id} title={titleOf(note)} onClick={() => onOpenNote(id)}>{titleOf(note)}</button>;
          }) : lastNote ? lastNoteItem ? <button title={titleOf(lastNoteItem)} onClick={() => onOpenNote(lastNote)}>{titleOf(lastNoteItem)}</button> : <span>삭제된 메모</span> : <span>쓰인 적 없음</span>}</div>
          <div className="asset-actions">
            <button title="크게 보기" onClick={openViewer}><Icon name="expand" size={16}/> 크게 보기</button>
            <button title="메모에 붙여 넣으면 같은 파일을 다시 써요" onClick={() => void copyReference(active)}><Icon name="copy" size={16}/> 참조 복사</button>
            {!active.usedBy.length && <button className="asset-delete" onClick={(event) => { confirmTrigger.current = event.currentTarget; setConfirm('one'); }}><Icon name="trash" size={16}/> 삭제…</button>}
          </div>
        </div>
      </aside>}
    </div>
    {message && <div className="toast" role="status">{message}<button aria-label="닫기" onClick={() => setMessage(null)}><Icon name="close" size={15}/></button></div>}
    {viewing && active && urls[active.id] && <ImageViewer src={urls[active.id]} alt={active.name} onClose={() => setViewing(false)}/>}
    {confirm && <ConfirmDialog
      question={confirm === 'one' ? '이 파일을 삭제할까요?' : `사용 안 함 파일 ${unused.length}개를 삭제할까요?`}
      detail={confirm === 'one' ? '삭제한 파일은 복구할 수 없어요.' : '삭제한 파일은 복구할 수 없어요. 메모에서 쓰고 있는 파일은 지우지 않아요.'}
      acceptLabel="삭제"
      onCancel={() => setConfirm(null)}
      onAccept={() => confirm === 'one' && active ? void removeOne(active) : void removeAll()}
    />}
  </div>;
}
