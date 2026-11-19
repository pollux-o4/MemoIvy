import type { StorybookConfig } from '@storybook/react-vite';

const config: StorybookConfig = {
  stories: ['../frontend/**/*.stories.@(ts|tsx)'],
  framework: { name: '@storybook/react-vite', options: {} },
  staticDirs: ['../docs/assets'],
};

export default config;
