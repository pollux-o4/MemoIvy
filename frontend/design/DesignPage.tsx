import type { ReactNode } from 'react';
import './design.css';

interface Props {
  eyebrow: string;
  title: string;
  /** 제목 아래 한 줄. Foundations는 값의 출처를 적는다. */
  lead: string;
  children: ReactNode;
}

/** 온보딩·기초 페이지 공통 틀. 본문 제목은 h2부터 시작한다. */
export function DesignPage({ eyebrow, title, lead, children }: Props) {
  return (
    // biome-ignore lint/a11y/noNoninteractiveTabindex: 스크롤 영역을 키보드로도 내릴 수 있어야 한다.
    <main className="dsb" tabIndex={0}>
      <header className="dsb-hero">
        <p className="dsb-eyebrow">{eyebrow}</p>
        <h1>{title}</h1>
        <p className="dsb-muted">{lead}</p>
      </header>
      {children}
    </main>
  );
}
