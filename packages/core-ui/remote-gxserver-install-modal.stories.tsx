import type { Meta, StoryObj } from '@storybook/react-vite';
import { useEffect } from 'react';
import { RemoteGxserverInstallModal } from './remote-gxserver-install-modal';

const noop = () => undefined;

/**
 * Install remote gxserver opens as a one-shot native fit-height modal in the
 * desktop app, so this story is its inspection surface. Switch the
 * "modalTheme" toolbar global to review the light appearance.
 */
function RemoteGxserverInstallModalStory({ machineName }: { machineName: string }) {
  return <RemoteGxserverInstallModal isOpen machineName={machineName} onApprove={noop} onCancel={noop} />;
}

const meta = {
  title: 'Modals/App Host/Remote Gxserver Install',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <RemoteGxserverInstallModalStory machineName='build-box' />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

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
      <RemoteGxserverInstallModalStory machineName='build-box' />
    </div>
  );
}

export const DesktopWindow: Story = {
  render: () => <DesktopWindowStory />,
};
