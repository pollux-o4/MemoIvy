import type { ReactNode } from 'react';
import { Routes } from './chapters/Routes';
import { SpecMap } from './chapters/SpecMap';
import { WindowMap } from './chapters/WindowMap';

export interface Chapter {
  id: string;
  title: string;
  lead: string;
  render: () => ReactNode;
}

/** Start Here의 장 등록표. 장을 더하려면 한 줄을 더한다. 세부 값은 Foundations 스토리로 내려보낸다. */
export const CHAPTERS: readonly Chapter[] = [
  { id: 'routes', title: '어디로 갈까', lead: '할 일을 고르면 사이드바 묶음과 코드 위치가 나온다', render: () => <Routes/> },
  { id: 'windows', title: '창 구조', lead: '화면마다 독립된 창이며 크기는 desktop/window_kinds.json이 정한다', render: () => <WindowMap/> },
  { id: 'specs', title: '명세 지도', lead: '동작·데이터·품질 계약. 고치기 전에 해당 ID를 docs에서 검색한다', render: () => <SpecMap/> },
];
