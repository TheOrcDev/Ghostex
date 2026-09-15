import { storageScope, storageFailure, subscribeStorage } from '@/packages/client-storage';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { MANAGE_DRAFTS_STORAGE_KEY_PREFIX, MANAGE_OPEN_FILES_STORAGE_KEY_PREFIX } from './constants';
import { isManageDescendantPath, isManageReviewDocumentPath, remapManagePathByMove } from './file-tree-utils';
import { isRecord } from './types';

const clientStorage = storageScope(["docsOpenFiles","docsDrafts"]);

/** An edited document that is open but not selected: its draft and the disk content it was edited from. */
export type ManageBackgroundDraft = {
  draft: string;
  savedContent: string;
};

export type ManageBackgroundDrafts = Record<string, ManageBackgroundDraft>;

function openFilesStorageKey(projectId: string): string {
  return `${MANAGE_OPEN_FILES_STORAGE_KEY_PREFIX}${projectId}`;
}

function draftsStorageKey(projectId: string): string {
  return `${MANAGE_DRAFTS_STORAGE_KEY_PREFIX}${projectId}`;
}

export function readStoredManageOpenFiles(projectId: string): string[] {
  try {
    const parsed: unknown = JSON.parse(clientStorage.getItem(openFilesStorageKey(projectId)) ?? '[]');
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.filter(
      (path): path is string => typeof path === 'string' && path.length > 0 && !isManageReviewDocumentPath(path)
    );
  } catch {
    return [];
  }
}

export function readStoredManageDrafts(projectId: string): ManageBackgroundDrafts {
  try {
    const parsed: unknown = JSON.parse(clientStorage.getItem(draftsStorageKey(projectId)) ?? '{}');
    if (!isRecord(parsed)) {
      return {};
    }
    const drafts: ManageBackgroundDrafts = {};
    for (const [path, value] of Object.entries(parsed)) {
      if (
        isRecord(value) &&
        typeof value.draft === 'string' &&
        typeof value.savedContent === 'string' &&
        value.draft !== value.savedContent
      ) {
        drafts[path] = { draft: value.draft, savedContent: value.savedContent };
      }
    }
    return drafts;
  } catch {
    return {};
  }
}

export function writeStoredManageDrafts(projectId: string, drafts: ManageBackgroundDrafts): void {
  try {
    if (Object.keys(drafts).length === 0) {
      clientStorage.removeItem(draftsStorageKey(projectId));
      return;
    }
    clientStorage.setItem(draftsStorageKey(projectId), JSON.stringify(drafts));
  } catch {
    // Storage quota or a private context: the draft still lives in memory for this page.
  }
}

/** The file name a tab shows for a path that the tree has not listed (a mounted or freshly created file). */
export function manageOpenFileLabel(path: string): string {
  const slash = path.lastIndexOf('/');
  return slash === -1 ? path : path.slice(slash + 1);
}

/**
 * CDXC:Docs 2026-09-15 DECISION:
 * User: every file opened in Docs gets a row in an open-files list above the tree, each open file keeps its own unsaved draft while another is selected, and drafts survive a quit or reload, still marked unsaved.
 * Open files and drafts are stored per project. A draft is applied to a file the next time it is read, on top of the disk content, so a file saved elsewhere with the same text drops its stale draft on its own.
 */
export function useManageOpenDocuments(projectId: string) {
  const persistenceError = useSyncExternalStore(subscribeStorage, () => storageFailure('docsDrafts'), () => undefined);
  const [openPaths, setOpenPaths] = useState<string[]>(() => readStoredManageOpenFiles(projectId));
  const [backgroundDrafts, setBackgroundDrafts] = useState<ManageBackgroundDrafts>(() =>
    readStoredManageDrafts(projectId)
  );
  const backgroundDraftsRef = useRef(backgroundDrafts);
  backgroundDraftsRef.current = backgroundDrafts;

  useEffect(() => {
    try {
      clientStorage.setItem(openFilesStorageKey(projectId), JSON.stringify(openPaths));
    } catch {
      // Same as drafts: the list is a convenience, never the source of truth.
    }
  }, [openPaths, projectId]);

  const openDocument = useCallback((path: string) => {
    if (isManageReviewDocumentPath(path)) {
      return;
    }
    setOpenPaths((current) => (current.includes(path) ? current : [...current, path]));
  }, []);

  const removeDocument = useCallback((path: string) => {
    setOpenPaths((current) => (current.includes(path) ? current.filter((candidate) => candidate !== path) : current));
    setBackgroundDrafts((current) => {
      if (!(path in current)) {
        return current;
      }
      const { [path]: _removed, ...rest } = current;
      backgroundDraftsRef.current = rest;
      return rest;
    });
  }, []);

  /** Drops every open file and draft at or under a deleted path. */
  const removeDeletedEntry = useCallback((deletedPath: string) => {
    const affected = (path: string) => path === deletedPath || isManageDescendantPath(path, deletedPath);
    setOpenPaths((current) => (current.some(affected) ? current.filter((path) => !affected(path)) : current));
    setBackgroundDrafts((current) => {
      if (!Object.keys(current).some(affected)) {
        return current;
      }
      const next: ManageBackgroundDrafts = {};
      for (const [path, draft] of Object.entries(current)) {
        if (!affected(path)) {
          next[path] = draft;
        }
      }
      backgroundDraftsRef.current = next;
      return next;
    });
  }, []);

  /** Follows a rename or move so the open row and its draft stay attached to the file. */
  const remapMovedEntry = useCallback((sourcePath: string, destinationPath: string) => {
    setOpenPaths((current) => {
      let changed = false;
      const next = current.map((path) => {
        const remapped = remapManagePathByMove(path, sourcePath, destinationPath);
        if (remapped === undefined) {
          return path;
        }
        changed = true;
        return remapped;
      });
      return changed ? next : current;
    });
    setBackgroundDrafts((current) => {
      let changed = false;
      const next: ManageBackgroundDrafts = {};
      for (const [path, draft] of Object.entries(current)) {
        const remapped = remapManagePathByMove(path, sourcePath, destinationPath);
        if (remapped !== undefined) {
          changed = true;
        }
        next[remapped ?? path] = draft;
      }
      if (!changed) {
        return current;
      }
      backgroundDraftsRef.current = next;
      return next;
    });
  }, []);

  const stashDraft = useCallback((path: string, draft: string, savedContent: string) => {
    setBackgroundDrafts((current) => {
      const next = { ...current, [path]: { draft, savedContent } };
      backgroundDraftsRef.current = next;
      return next;
    });
  }, []);

  /** Returns and forgets the stored draft for a path, synchronously, so a read can apply it in the same tick. */
  const takeDraft = useCallback((path: string): ManageBackgroundDraft | undefined => {
    const draft = backgroundDraftsRef.current[path];
    if (!draft) {
      return undefined;
    }
    const { [path]: _taken, ...rest } = backgroundDraftsRef.current;
    backgroundDraftsRef.current = rest;
    setBackgroundDrafts(rest);
    return draft;
  }, []);

  const backgroundDirtyPaths = useMemo(() => new Set(Object.keys(backgroundDrafts)), [backgroundDrafts]);

  return {
    storageError: persistenceError ? 'Your document edits could not be saved on this computer. Keep this view open until saving succeeds.' : '',
    backgroundDirtyPaths,
    backgroundDrafts,
    openDocument,
    openPaths,
    remapMovedEntry,
    removeDeletedEntry,
    removeDocument,
    stashDraft,
    takeDraft,
  };
}
