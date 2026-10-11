import { type BinaryFiles } from '@excalidraw/excalidraw/types';
import { type ExcalidrawElement } from '@excalidraw/excalidraw/element/types';
import { type ProjectDocsRequest as ManageFilesBridgeRequest } from '@/packages/shared/project-docs';

export type ManageWebKitWindow = Window & {
  ghostexGpui?: {
    manageDocsResourceBaseUrl?: string;
    postManageFilesRequest?: (payload: string) => boolean;
  };
  webkit?: {
    messageHandlers?: {
      ghostexManageFiles?: {
        postMessage: (message: ManageFilesBridgeRequest) => void;
      };
    };
  };
};

export type ExcalidrawFileData = {
  appState?: Record<string, unknown>;
  elements?: readonly ExcalidrawElement[];
  files?: BinaryFiles;
  source?: string;
  type?: string;
  version?: number;
};

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
