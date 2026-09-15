import { type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent } from 'react';
import { IconX } from '@tabler/icons-react';
import { type ProjectDocsFileEntry as ManageFileEntry } from '@/packages/shared/project-docs';
import {
  AppModalButton,
  AppModalDescription,
  AppModalFooter,
  AppModalHeader,
  AppModalShell,
  AppModalStack,
  AppModalTitle,
} from '@/packages/core-ui/app-modal-shell';
import { AppTooltip } from '@/packages/core-ui/app-tooltip';
import { fileIconForPath } from './file-tree-utils';
import { manageOpenFileLabel } from './open-documents';

/**
 * CDXC:Docs 2026-09-15 DECISION:
 * User: a vertical list of the currently open files sits under Search and above the tree, and is exactly as tall as the number of open files.
 * Each row is the file's icon and name; hovering shows a close control, and an unsaved file shows a dot in its place until hovered, so the list doubles as the unsaved indicator.
 */
export function ManageOpenFilesList({
  dirtyPaths,
  entriesByPath,
  onClose,
  onSelect,
  openPaths,
  selectedPath,
}: {
  dirtyPaths: ReadonlySet<string>;
  entriesByPath: ReadonlyMap<string, ManageFileEntry>;
  onClose: (path: string) => void;
  onSelect: (path: string) => void;
  openPaths: readonly string[];
  selectedPath: string | undefined;
}) {
  if (openPaths.length === 0) {
    return null;
  }

  const handleRowKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>, path: string) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onSelect(path);
      return;
    }
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      onClose(path);
    }
  };

  const handleRowAuxClick = (event: ReactMouseEvent<HTMLDivElement>, path: string) => {
    if (event.button === 1) {
      event.preventDefault();
      onClose(path);
    }
  };

  return (
    <div aria-label='Open files' aria-orientation='vertical' className='manage-open-files' role='tablist'>
      {openPaths.map((path) => {
        const entry = entriesByPath.get(path);
        const label = entry?.name ?? manageOpenFileLabel(path);
        const displayPath = entry?.displayPath ?? path;
        const isSelected = path === selectedPath;
        const isDirty = dirtyPaths.has(path);
        const Icon = fileIconForPath(path);
        return (
          <div
            aria-selected={isSelected}
            className='manage-open-file-row'
            data-dirty={String(isDirty)}
            data-selected={String(isSelected)}
            key={path}
            onAuxClick={(event) => handleRowAuxClick(event, path)}
            onClick={() => onSelect(path)}
            onKeyDown={(event) => handleRowKeyDown(event, path)}
            role='tab'
            tabIndex={isSelected ? 0 : -1}
            title={displayPath}
          >
            <Icon aria-hidden='true' className='manage-file-icon' size={15} stroke={1.75} />
            <span className='manage-open-file-name'>
              <bdi>{label}</bdi>
            </span>
            <AppTooltip content={isDirty ? 'Unsaved changes. Close file' : 'Close file'}>
              <button
                aria-label={isDirty ? `Close ${label} (unsaved changes)` : `Close ${label}`}
                className='manage-open-file-close'
                onClick={(event) => {
                  event.stopPropagation();
                  onClose(path);
                }}
                tabIndex={-1}
                type='button'
              >
                <span aria-hidden='true' className='manage-open-file-dot' />
                <IconX aria-hidden='true' size={14} stroke={1.8} />
              </button>
            </AppTooltip>
          </div>
        );
      })}
    </div>
  );
}

/**
 * CDXC:Docs 2026-09-15 DECISION:
 * User: closing an open file that has unsaved changes asks first, with Save, Discard, and Cancel, the way familiar editors do.
 */
export function ManageCloseDocumentDialog({
  error,
  isSaving,
  label,
  onCancel,
  onDiscard,
  onSave,
}: {
  error?: string;
  isSaving: boolean;
  label: string;
  onCancel: () => void;
  onDiscard: () => void;
  onSave: () => void;
}) {
  return (
    <AppModalShell className='manage-close-document-modal' isOpen onClose={onCancel}>
      <AppModalStack>
        <AppModalHeader>
          <AppModalTitle>Save changes to {label}?</AppModalTitle>
          <AppModalDescription>
            {error ?? 'Your changes will be lost if you close the file without saving.'}
          </AppModalDescription>
        </AppModalHeader>
        <AppModalFooter>
          <AppModalButton disabled={isSaving} onClick={onCancel} type='button'>
            Cancel
          </AppModalButton>
          <AppModalButton disabled={isSaving} onClick={onDiscard} tone='danger' type='button'>
            Discard
          </AppModalButton>
          <AppModalButton disabled={isSaving} onClick={onSave} tone='primary' type='button'>
            {isSaving ? 'Saving' : 'Save'}
          </AppModalButton>
        </AppModalFooter>
      </AppModalStack>
    </AppModalShell>
  );
}
