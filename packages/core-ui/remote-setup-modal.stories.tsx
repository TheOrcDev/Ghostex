import type { Meta, StoryObj } from '@storybook/react-vite';
import { RemoteSetupModal, type RemoteSetupRpc } from './remote-setup-modal';

const noop = () => undefined;

const wait = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

/** Answers the three Easy Connect calls after a short delay, like a healthy gxserver. */
const HEALTHY_RPC: RemoteSetupRpc = async (path) => {
  await wait(1200);
  switch (path) {
    case '/api/remoteAccessStatus':
      return { ssh: { enabled: true } };
    case '/api/enableSshAccess':
      return { outcome: 'enabled' };
    default:
      return {};
  }
};

/** Connecting never settles, which keeps the spinner visible. */
const PENDING_RPC: RemoteSetupRpc = () => new Promise(() => undefined);

/** SSH is off and turning it on fails. */
const FAILING_RPC: RemoteSetupRpc = async (path) => {
  await wait(300);
  if (path === '/api/remoteAccessStatus') {
    return { ssh: { enabled: false } };
  }
  return {
    message: 'SSH access could not be turned on. Remote Login is managed by your organization.',
    outcome: 'failed',
  };
};

function RemoteSetupModalStory({
  rpc = HEALTHY_RPC,
  tailscaleEnabled = true,
}: {
  rpc?: RemoteSetupRpc;
  tailscaleEnabled?: boolean;
}) {
  return (
    <RemoteSetupModal isOpen onClose={noop} onOpenExternalUrl={noop} rpc={rpc} tailscaleEnabled={tailscaleEnabled} />
  );
}

/**
 * The sidebar menu's Mobile & Remote dialog. Switch the "modalTheme" toolbar
 * global to review the light appearance. Click "How to install" to open the
 * Android popover and "Connect" to see the connecting state.
 */
const meta = {
  title: 'Modals/App Host/Remote Setup',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <RemoteSetupModalStory />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

/** Connect stays in its connecting state. */
export const Connecting: Story = {
  render: () => <RemoteSetupModalStory rpc={PENDING_RPC} />,
};

/** Connect fails while turning SSH access on. */
export const ConnectError: Story = {
  render: () => <RemoteSetupModalStory rpc={FAILING_RPC} />,
};

/** No gxserver is reachable, so Connect is disabled. */
export const NoServer: Story = {
  render: () => <RemoteSetupModalStory rpc={undefined} />,
};

/** `remoteTailscaleEnabled` is off, so only Easy Connect is offered. */
export const TailscaleHidden: Story = {
  render: () => <RemoteSetupModalStory tailscaleEnabled={false} />,
};
