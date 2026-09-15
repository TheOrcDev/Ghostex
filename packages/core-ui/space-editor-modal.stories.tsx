import type { Meta, StoryObj } from '@storybook/react-vite';
import { SpaceEditorModal } from './space-editor-modal';

const noop = () => undefined;

/**
 * New Space / Edit Space opens as a one-shot native fit-height modal in the
 * desktop app, so these stories are its inspection surface. Switch the
 * "modalTheme" toolbar global to review the light appearance.
 */
function SpaceEditorModalStory({
  initialColor,
  initialIcon,
  initialName,
  mode,
}: {
  initialColor?: string;
  initialIcon?: string;
  initialName?: string;
  mode: 'create' | 'edit';
}) {
  return (
    <SpaceEditorModal
      initialColor={initialColor}
      initialIcon={initialIcon}
      initialName={initialName}
      isOpen
      mode={mode}
      onCancel={noop}
      onDelete={noop}
      onSubmit={noop}
    />
  );
}

const meta = {
  title: 'Modals/App Host/Space Editor',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <SpaceEditorModalStory mode='create' />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Create: Story = {};

/** Edit mode: the row's current name, icon and color, plus the Delete action. */
export const Edit: Story = {
  render: () => (
    <SpaceEditorModalStory initialColor='#3f8fc7' initialIcon='rocket' initialName='Client work' mode='edit' />
  ),
};
