import type { ReactNode } from 'react';

export function EditorContent({ children, onFile }: { children: ReactNode; onFile: (file: File, drop?: { x: number; y: number }) => void }) {
  // biome-ignore lint/a11y/noStaticElementInteractions: 붙여넣기·끌어 놓기로 파일을 받는 영역이다. 입력 자체는 안쪽 편집기가 맡는다.
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
