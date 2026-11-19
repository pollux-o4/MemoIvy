// 스토리북 탐색 계층의 단일 출처. 온보딩 갈림길이 여기서 나오고, 스토리 제목·preview 정렬은 hierarchy.test가 맞춘다.
// 위에서 아래로: 시작 → 화면(창 상태) → 조작(재현 가능한 상호작용).

export interface Section {
  root: 'Onboarding' | 'Screens' | 'Interactions';
  /** 이 묶음에 속하는 스토리 파일 위치. 제목만 바꿔 다른 묶음으로 옮길 수 없다. */
  from: RegExp;
  /** 온보딩 갈림길의 질문. "~하려면" 꼴로 할 일을 말한다. */
  purpose: string;
  /** 작업할 코드·자산 위치. 테스트가 실제로 있는지 확인한다. */
  codePath: string;
}

export const SECTIONS: readonly Section[] = [
  { root: 'Onboarding', from: /^frontend\/design\//, purpose: '색·글자 토큰과 문서 지도를 먼저 보려면', codePath: 'frontend/design' },
  { root: 'Screens', from: /^frontend\/app\/App\.stories\.tsx$/, purpose: '창 화면을 상태별로 살펴보려면', codePath: 'frontend/features' },
  { root: 'Interactions', from: /^frontend\/app\/Interactions\.stories\.tsx$/, purpose: '펼침·키보드·저장 실패 같은 조작을 재현하려면', codePath: 'frontend/app' },
];

/** 파일 위치에 맞는 최상위 묶음. 어느 묶음에도 속하지 않으면 undefined. */
export const sectionOf = (path: string) => SECTIONS.find((section) => section.from.test(path))?.root;
