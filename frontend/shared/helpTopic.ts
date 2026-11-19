export type HelpTopic = 'note' | 'canvas';
const KEY = 'memoivy:help-topic';

export function readHelpTopic(): HelpTopic {
  try { return localStorage.getItem(KEY) === 'canvas' ? 'canvas' : 'note'; } catch { return 'note'; }
}

export function rememberHelpTopic(topic: HelpTopic) {
  try { localStorage.setItem(KEY, topic); } catch { /* 저장 못 해도 기본 주제로 연다 */ }
}

export const HELP_TOPIC_KEY = KEY;
