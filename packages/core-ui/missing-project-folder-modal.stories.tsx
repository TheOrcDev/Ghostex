import type { Meta, StoryObj } from '@storybook/react-vite';
import { useEffect } from 'react';
import { MissingProjectFolderModal } from './missing-project-folder-modal';

const noop = () => undefined;

/**
 * Missing Project Folder opens as a one-shot native fit-height modal in the
 * desktop app, so this story is its inspection surface. Switch the
 * "modalTheme" toolbar global to review the light appearance.
 */
function MissingProjectFolderModalStory({ projectPath }: { projectPath: string }) {
  return (
    <MissingProjectFolderModal
      isOpen
      onCancel={noop}
      onLocate={noop}
      onRemove={noop}
      projectName='Ghostex'
      projectPath={projectPath}
    />
  );
}

const meta = {
  title: 'Modals/App Host/Missing Project Folder',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <MissingProjectFolderModalStory projectPath='/Users/story/dev/_active/Ghostex' />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

/** A path longer than the card clips with an ellipsis instead of wrapping. */
export const LongPath: Story = {
  render: () => (
    <MissingProjectFolderModalStory projectPath='/Volumes/External Drive/Archive/2026/clients/acme-corporation/platform/services/ghostex-monorepo-with-a-very-long-name' />
  ),
};

/**
 * The desktop app opens this modal in a 560px-wide native child window that
 * fits its height to the content, so this variant applies the native body
 * class and lets the dialog fill the canvas the way the app renders it.
 */
function DesktopWindowStory() {
  useEffect(() => {
    document.body.classList.add('app-modal-host-native-window-body');
    return () => {
      document.body.classList.remove('app-modal-host-native-window-body');
    };
  }, []);
  return (
    <div style={{ width: 560 }}>
      <MissingProjectFolderModalStory projectPath='/Users/story/dev/_active/Ghostex' />
    </div>
  );
}

export const DesktopWindow: Story = {
  render: () => <DesktopWindowStory />,
};
