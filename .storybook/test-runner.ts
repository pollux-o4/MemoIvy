import type { TestRunnerConfig } from '@storybook/test-runner';
import { checkA11y, injectAxe } from 'axe-playwright';

/** 모든 스토리의 play 상호작용 뒤 axe 접근성 검사를 돌린다. GTK 원본 PNG 비교 화면은 이미지 자체라 제외한다. */
const config: TestRunnerConfig = {
  async preVisit(page) { await injectAxe(page); },
  async postVisit(page, context) {
    if (context.title.startsWith('Reference/')) return;
    await checkA11y(page, '#storybook-root', { detailedReport: true, detailedReportOptions: { html: true } });
  },
};
export default config;
