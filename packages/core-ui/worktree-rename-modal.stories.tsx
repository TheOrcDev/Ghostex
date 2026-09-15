import type { Meta, StoryObj } from '@storybook/react-vite';
import { useContext } from 'react';
import { ModalStoryTheme } from './modal-gallery/modal-story-surface';
import { WorktreeRenameModal, type WorktreeRenameModalDraft } from './worktree-rename-modal';

const noop = () => undefined;

const RENAME_DRAFT: WorktreeRenameModalDraft = {
  branch: 'feat/modal-gallery',
  currentName: 'feat-modal-gallery',
  currentPath: '/Users/you/dev/Ghostex-feat-modal-gallery',
  parentFolderName: 'Ghostex',
  parentProjectPath: '/Users/you/dev/Ghostex',
  projectId: 'story-worktree-project',
  registeredProjectPaths: ['/Users/you/dev/Ghostex-settings-redesign'],
  renameBranchDefault: true,
  warnings: ['A remote branch already exists and will keep its current name.'],
  worktreeName: 'Ghostex-feat-modal-gallery',
};

/**
 * Rename Worktree is a one-shot native fit-height modal in the desktop app,
 * so these stories are its inspection surface. Switch the "modalTheme"
 * toolbar global to review the light appearance; the dialog reads it so its
 * shadcn `dark` class follows the surface. Type into the field to see the
 * validation and collision refusals (`settings-redesign` collides).
 */
function WorktreeRenameModalStory({ draft = RENAME_DRAFT }: { draft?: WorktreeRenameModalDraft }) {
  const appearance = useContext(ModalStoryTheme);
  return (
    <WorktreeRenameModal
      draft={draft}
      isOpen
      onCancel={noop}
      onRename={noop}
      theme={appearance === 'light' ? 'plain-light' : 'dark-2'}
    />
  );
}

const meta = {
  title: 'Modals/App Host/Rename Worktree',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <WorktreeRenameModalStory />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

/** The branch prefill, branch rename on, one warning. */
export const Default: Story = {};

/** A reason that keeps Rename disabled while the field stays editable. */
export const Blocking: Story = {
  render: () => (
    <WorktreeRenameModalStory
      draft={{
        ...RENAME_DRAFT,
        blockingReason: 'This worktree has populated submodules; rename it after deinitializing them.',
      }}
    />
  ),
};

/** A detached worktree: the folder suffix is the prefill and branch rename is off. */
export const NoBranch: Story = {
  render: () => (
    <WorktreeRenameModalStory
      draft={{ ...RENAME_DRAFT, branch: undefined, renameBranchDefault: false, warnings: [] }}
    />
  ),
};

/** No warnings at all. */
export const Plain: Story = {
  render: () => <WorktreeRenameModalStory draft={{ ...RENAME_DRAFT, warnings: [] }} />,
};
