import type { ReactNode } from 'react';

export function EditorContent({ children, onFile }: { children: ReactNode; onFile: (file: File, drop?: { x: number; y: number }) => void }) {
  return <div className="editor-content"
    onPaste={(event) => {
      if (event.clipboardData.getData('text/plain')) return;
      const file = Array.from(event.clipboardData.items).map((item) => item.getAsFile()).find((item) => item?.type.startsWith('image/'));
      if (file) { event.preventDefault(); onFile(file); }
    }}
    onDragOver={(event) => { if (event.dataTransfer.files.length) event.preventDefault(); }}
    onDrop={(event) => {
      const file = Array.from(event.dataTransfer.files).find((item) => item.type.startsWith('image/'));
      if (file) { event.preventDefault(); onFile(file, { x: event.clientX, y: event.clientY }); }
    }}>
    {children}
  </div>;
}
