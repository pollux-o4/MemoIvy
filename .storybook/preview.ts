import '../frontend/styles.css';
import type { Preview } from '@storybook/react-vite';

const preview: Preview = {
  parameters: {
    layout: 'fullscreen',
    controls: { expanded: true },
    backgrounds: { disable: true },
    // Storybook이 정적으로 읽으므로 인라인으로 둔다. 최상위 순서는 frontend/design/hierarchy.ts의 SECTIONS와 같아야 한다(hierarchy.test).
    options: {
      storySort: {
        order: [
          'Onboarding',
          ['Start Here', 'Foundations', ['Tokens']],
          'Screens',
          'Interactions',
          'Reference',
        ],
      },
    },
  },
};

export default preview;
