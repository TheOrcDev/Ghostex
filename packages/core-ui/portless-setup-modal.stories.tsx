import type { Meta, StoryObj } from '@storybook/react-vite';
import { useEffect } from 'react';
import { PortlessSetupModal, type PortlessSetupModalMode } from './portless-setup-modal';

const noop = () => undefined;

/**
 * Portless Setup opens as a one-shot native fit-height modal in the desktop
 * app, so these stories are its inspection surface. Switch the "modalTheme"
 * toolbar global to review the light appearance.
 */
function PortlessSetupModalStory({ mode }: { mode: PortlessSetupModalMode }) {
  return (
    <PortlessSetupModal
      isOpen
      mode={mode}
      onAdminAction={noop}
      onCancel={noop}
      onDisable={noop}
      onPostpone={noop}
      protocol='https'
    />
  );
}

const meta = {
  title: 'Modals/App Host/Portless Setup',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <PortlessSetupModalStory mode='firstSetup' />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

/** First run: Postpone / Disable / Install. */
export const FirstSetup: Story = {};

/** Portless already installed by something else: Cancel / Disable / Reconfigure. */
export const StandaloneReconfigure: Story = {
  render: () => <PortlessSetupModalStory mode='standaloneReconfigure' />,
};

/**
 * The desktop app opens this modal in a 640px-wide native child window that
 * fits its height to the content, so this variant applies the native body
 * class and lets the dialog fill the canvas the way the app renders it.
 */
function DesktopWindowStory({ mode }: { mode: PortlessSetupModalMode }) {
  useEffect(() => {
    document.body.classList.add('app-modal-host-native-window-body');
    return () => {
      document.body.classList.remove('app-modal-host-native-window-body');
    };
  }, []);
  return (
    <div style={{ width: 640 }}>
      <PortlessSetupModalStory mode={mode} />
    </div>
  );
}

export const DesktopWindow: Story = {
  render: () => <DesktopWindowStory mode='firstSetup' />,
};

export const DesktopWindowReconfigure: Story = {
  render: () => <DesktopWindowStory mode='standaloneReconfigure' />,
};
