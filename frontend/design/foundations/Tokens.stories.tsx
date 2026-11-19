import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { DesignPage } from '../DesignPage';
import { TokenPanel } from './TokenPanel';

function Tokens() {
  return (
    <DesignPage eyebrow="FOUNDATIONS" title="토큰" lead="앱 표면·메모 색·글자. 값은 frontend/shared/tokens.css에서 읽고, 견본은 왼쪽 라이트·오른쪽 다크다.">
      <TokenPanel/>
    </DesignPage>
  );
}

const meta = {
  title: 'Onboarding/Foundations/Tokens',
  component: Tokens,
  parameters: { layout: 'fullscreen' },
} satisfies Meta<typeof Tokens>;

export default meta;
type Story = StoryObj<typeof meta>;

export const All: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('region', { name: '앱 표면 토큰' })).toHaveTextContent('--memoivy-accent');
    await expect(canvas.getByRole('region', { name: '메모 색 토큰' })).toHaveTextContent('--note-yellow');
    await expect(canvas.getByRole('region', { name: '글자 토큰' })).toHaveTextContent('--font-body');
  },
};
