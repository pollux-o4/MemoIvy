import { useEffect, useState } from 'react';
import { Icon } from './Icon';
import { usePresence } from './menu/usePresence';
import './image_viewer.css';

/** GTK 이미지 크게 보기: 클릭으로 원래 크기 전환, 바깥·Esc로 퇴장 모션 후 닫기. */
export function ImageViewer({ src, alt, onClose }: { src: string; alt: string; onClose: () => void }) {
  const presence = usePresence(120, 'open');
  const [originalSize, setOriginalSize] = useState(false);
  const { hide, mounted } = presence;
  useEffect(() => { if (!mounted) onClose(); }, [mounted, onClose]);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); hide(); } };
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [hide]);
  return <div className={`asset-viewer${presence.state === 'closing' ? ' leaving' : ''}`} role="dialog" aria-label="이미지 크게 보기" aria-modal="true" onClick={hide}>
    <div className="asset-viewer-stage">
      <button aria-label="닫기" title="닫기 · Esc" onClick={(event) => { event.stopPropagation(); hide(); }}><Icon name="close"/></button>
      <img className={originalSize ? 'original-size' : ''} src={src} alt={alt} onClick={(event) => { event.stopPropagation(); setOriginalSize((value) => !value); }}/>
    </div>
  </div>;
}
