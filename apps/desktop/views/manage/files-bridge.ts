import {
  requestProjectDocsFromHost,
  type ProjectDocsRequest as ManageFilesBridgeRequest,
  type ProjectDocsResponse as ManageFilesBridgeResponse,
} from '@/packages/shared/project-docs';
import { MANAGE_BRIDGE_TIMEOUT_MS, MANAGE_FILES_RESPONSE_EVENT } from './constants';
import type { ManageWebKitWindow } from './types';

export function requestManageFiles(
  request: Omit<ManageFilesBridgeRequest, 'requestId'>
): Promise<ManageFilesBridgeResponse> {
  const bridge = (window as ManageWebKitWindow).webkit?.messageHandlers?.ghostexManageFiles;
  if (!bridge) {
    return Promise.reject(new Error('Files is unavailable in this host.'));
  }
  return requestProjectDocsFromHost(request, {
    eventName: MANAGE_FILES_RESPONSE_EVENT,
    eventTarget: window,
    postMessage: (message) => bridge.postMessage(message),
    timeoutMs: MANAGE_BRIDGE_TIMEOUT_MS,
  });
}
