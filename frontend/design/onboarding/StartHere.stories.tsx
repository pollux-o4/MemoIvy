import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import windowKinds from '../../../desktop/window_kinds.json';
import { SECTIONS } from '../hierarchy';
import { CHAPTERS } from './chapters';
import { StartHere } from './StartHere';

const meta = {
  title: 'Onboarding/Start Here',
  component: StartHere,
  parameters: { layout: 'fullscreen' },
} satisfies Meta<typeof StartHere>;

export default meta;
type Story = StoryObj<typeof meta>;

/** 장과 갈림길이 실제 출처(계층·창 종류·docs/specs)에서 채워진다. */
export const Overview: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    for (const chapter of CHAPTERS) {
      await expect(canvas.getByRole('heading', { level: 2, name: chapter.title })).toBeVisible();
    }
    for (const section of SECTIONS) {
      await expect(canvas.getByRole('listitem', { name: section.root })).toHaveTextContent(section.codePath);
    }
    for (const kind of Object.keys(windowKinds)) {
      await expect(canvas.getByRole('listitem', { name: kind })).toBeVisible();
    }
    await expect(canvas.getByRole('listitem', { name: 'quality' })).toHaveTextContent('UI-001');
  },
};
