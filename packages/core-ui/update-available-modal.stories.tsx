import type { Meta, StoryObj } from '@storybook/react-vite';
import { UpdateAvailableModal, type UpdateAvailableModalState } from './update-available-modal';

const noop = () => undefined;

const RELEASE_NOTES = `# Ghostex 9.6.0

## 9.6.0 - 2026-09-15

- New Features
  - Your phone and your Mac can now drive a Windows computer. Install Ghostex on Windows, turn on SSH, and add the Windows address with your Windows username.
  - Install and update agent CLIs from **Settings > Agents**. Expand an agent row to install or update it, see its installed version and the command output, or open its install docs.
  - Start a chat message with \`!\` to run a shell command inside a Claude Code or Codex session, for example \`! pwd\`, with the command and its output shown in the conversation.
  - Ghostex is in the official Homebrew cask, so \`brew install ghostex\` works without adding a tap first. See [the release page](https://github.com/maddada/ghostex/releases) for the full list.
- Major Improvements
  - Scrollbars look the same everywhere. A thin 5px track floats over the content, appears when you are near it, and reserves no gutter.
  - The phone reconnects on its own. With **Auto reconnect** on, it checks the connection when you return to the app or its network changes.

> Sessions started from a custom profile pick up confirmed renames instead of staying pending.

## 9.5.2 - 2026-09-08

- Stabilization
  - Codex questions asked mid-turn stay answerable after the turn ends.
  - One dead remote tunnel can no longer stall reconnects to every other computer.
  - Claude keeps its brand color on light notification tiles.
`;

function UpdateAvailableModalStory({
  notesMarkdown = RELEASE_NOTES,
  portable = false,
  state = 'available',
}: Partial<UpdateAvailableModalState>) {
  return (
    <UpdateAvailableModal
      isOpen
      onCancel={noop}
      onDownload={noop}
      onRestart={noop}
      update={{ notesMarkdown, portable, state, version: '9.6.0' }}
    />
  );
}

/**
 * The Windows updater's release dialog. Switch the "modalTheme" toolbar global
 * to review the light appearance.
 */
const meta = {
  title: 'Modals/App Host/Update Available',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <UpdateAvailableModalStory />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Available: Story = {};

/** The update is downloaded and waits for a restart. */
export const Ready: Story = {
  render: () => <UpdateAvailableModalStory state='ready' />,
};

/** A portable install adds the in-place note under the release notes. */
export const Portable: Story = {
  render: () => <UpdateAvailableModalStory portable state='ready' />,
};

/** No release notes: the dialog shows the fallback sentence. */
export const NoNotes: Story = {
  render: () => <UpdateAvailableModalStory notesMarkdown='' />,
};
