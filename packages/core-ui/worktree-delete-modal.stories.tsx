import type { Meta, StoryObj } from '@storybook/react-vite';
import { useContext } from 'react';
import { ModalStoryTheme } from './modal-gallery/modal-story-surface';
import { WorktreeDeleteModal, type WorktreeDeleteModalDraft } from './worktree-delete-modal';

const noop = () => undefined;

const CHANGES_DRAFT: WorktreeDeleteModalDraft = {
  branch: 'feat/modal-gallery',
  canDeleteLocalBranch: true,
  groupId: 'story-worktree-group',
  hasChanges: true,
  localBranchName: 'feat/modal-gallery',
  projectId: 'story-worktree-project',
  remoteBranchExists: true,
  remoteBranchName: 'feat/modal-gallery',
  remoteName: 'origin',
  statusSummary: [
    ' M packages/core-ui/styles/modals.css',
    ' M apps/desktop/src/app/window/native_modal_kit.rs',
    '?? packages/core-ui/modal-gallery/',
    '?? apps/desktop/assets/modals/delete-worktree/',
  ].join('\n'),
  worktreeName: 'Ghostex-modal-gallery',
};

/**
 * Delete Worktree is a one-shot native fit-height modal in the desktop app,
 * so these stories are its inspection surface. Switch the "modalTheme"
 * toolbar global to review the light appearance; the dialog reads it so its
 * shadcn `dark` class follows the surface.
 */
function WorktreeDeleteModalStory({ draft = CHANGES_DRAFT }: { draft?: WorktreeDeleteModalDraft }) {
  const appearance = useContext(ModalStoryTheme);
  return (
    <WorktreeDeleteModal
      draft={draft}
      isOpen
      onCancel={noop}
      onCommit={noop}
      onDelete={noop}
      theme={appearance === 'light' ? 'plain-light' : 'dark-2'}
    />
  );
}

const meta = {
  title: 'Modals/App Host/Delete Worktree',
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <WorktreeDeleteModalStory />,
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

/** Uncommitted changes: the status summary and the Commit button. */
export const WithChanges: Story = {};

/** A clean checkout: the green check row, no Commit button. */
export const Clean: Story = {
  render: () => <WorktreeDeleteModalStory draft={{ ...CHANGES_DRAFT, hasChanges: false, statusSummary: '' }} />,
};

/** Neither branch can be deleted: both rows muted, the daemon's remote reason shown. */
export const BranchesDisabled: Story = {
  render: () => (
    <WorktreeDeleteModalStory
      draft={{
        ...CHANGES_DRAFT,
        canDeleteLocalBranch: false,
        remoteBranchDisabledReason: 'The remote branch has commits that are not in this worktree.',
        remoteBranchExists: false,
      }}
    />
  ),
};

/** A status summary long enough to scroll inside its 220px box. */
export const LongStatus: Story = {
  render: () => (
    <WorktreeDeleteModalStory
      draft={{
        ...CHANGES_DRAFT,
        statusSummary: Array.from({ length: 40 }, (_, index) => ` M packages/core-ui/styles/file-${index}.css`).join(
          '\n'
        ),
      }}
    />
  ),
};
