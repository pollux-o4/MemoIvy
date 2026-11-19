import { useEffect, useEffectEvent, useRef, useState } from 'react';
import type { MemoStore } from './useMemoivy';

/** base64 응답을 Blob URL로 바꾼다. */
function blobUrl(data: string, mime: string): string {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return URL.createObjectURL(new Blob([bytes], { type: mime }));
}

/**
 * 첨부 파일 id → 화면에 띄울 Blob URL. 편집기·파일 창이 함께 쓰는 단일 출처다.
 * 첨부가 늘면 새 것만 읽고, 화면을 떠나면 만든 URL을 모두 해제한다.
 */
export function useAssetUrls(store: MemoStore): Record<string, string> {
  const [urls, setUrls] = useState<Record<string, string>>({});
  const requested = useRef(new Set<string>());
  const mounted = useRef(false);
  const created = useRef<string[]>([]);
  // 첨부 바이트는 읽기 전용이라 편집 창의 쓰기 큐 교체와 독립적으로 읽는다.
  const read = useEffectEvent((id: string) => store.bridge.request('readAsset', { id }));

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      for (const url of created.current) URL.revokeObjectURL(url);
    };
  }, []);

  useEffect(() => {
    for (const asset of store.assets) {
      if (requested.current.has(asset.id)) continue;
      requested.current.add(asset.id);
      void read(asset.id).then((result) => {
        if (!mounted.current || !result.data) { requested.current.delete(asset.id); return; }
        const url = blobUrl(result.data, result.mime ?? asset.mime);
        created.current.push(url);
        setUrls((current) => ({ ...current, [asset.id]: url }));
      }).catch(() => { requested.current.delete(asset.id); });
    }
  }, [store.assets]);

  return urls;
}
