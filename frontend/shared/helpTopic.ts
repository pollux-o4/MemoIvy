import type { DesktopBridge } from './engine_contract';

/** 도움말 첫 화면 주제. 연 창의 맥락(캔버스·메모)을 창 사이에 localStorage로 넘긴다(같은 출처라 열린 도움말 창도 storage 이벤트로 따라온다). */
export type HelpTopic = 'note' | 'canvas';
const KEY = 'memoivy:help-topic';

export function readHelpTopic(): HelpTopic {
  try { return localStorage.getItem(KEY) === 'canvas' ? 'canvas' : 'note'; } catch { return 'note'; }
}

export function rememberHelpTopic(topic: HelpTopic) {
  try { localStorage.setItem(KEY, topic); } catch { /* 저장 못 해도 기본 주제로 연다 */ }
}

/** 주제를 남기고 도움말 창을 연다. */
export function openHelpWindow(bridge: DesktopBridge, topic: HelpTopic) {
  rememberHelpTopic(topic);
  return bridge.openWindow('help');
}

export const HELP_TOPIC_KEY = KEY;
