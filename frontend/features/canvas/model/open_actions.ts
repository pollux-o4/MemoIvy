import type { IconName } from '../../../shared/Icon';

/** 카드를 여는 동작 한 가지. 카드 메뉴·인스펙터가 이 목록 하나로 이름·아이콘·단축키를 그린다. */
export interface OpenAction {
  id: 'open' | 'openAsNote';
  icon: IconName;
  label: string;
  /** aria-keyshortcuts 표기. 화면 칩은 keyCap으로 줄인다. */
  shortcut: string;
  /** 카드를 두 번 눌러도 같은 동작이다(첫 동작만). */
  doubleClick: boolean;
}

const OPEN_NOTE: OpenAction = { id: 'open', icon: 'note', label: '메모 열기', shortcut: 'Enter', doubleClick: true };
const ENTER_PROJECT: OpenAction = { id: 'open', icon: 'enter', label: '프로젝트 들어가기', shortcut: 'Enter', doubleClick: true };
const PROJECT_AS_NOTE: OpenAction = { id: 'openAsNote', icon: 'note', label: '메모로 열기', shortcut: 'Shift+Enter', doubleClick: false };

/** 주 동작이 먼저 온다. 프로젝트는 들어가기 + 소개를 메모 편집기로 열기, 메모는 열기 하나. */
export const openActions = (project: boolean): readonly OpenAction[] => project ? [ENTER_PROJECT, PROJECT_AS_NOTE] : [OPEN_NOTE];

/** 단축키 칩 표기: Shift는 ⇧ 기호로 줄인다. */
export const keyCap = (shortcut: string) => shortcut.replace(/^Shift\+/, '⇧ ');
