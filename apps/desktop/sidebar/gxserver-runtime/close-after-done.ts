/*
CDXC:RepoStructure 2026-08-22:
Split out of the single 21,861-line `gxserver-runtime.ts`. Pure move: no logic
changed. See `core.ts` for how the runtime's methods are re-attached.
*/
import {
  GPUI_CLOSE_AFTER_DONE_DELAY_MS,
  GPUI_DELAYED_SEND_MAX_DELAY_MS,
} from './constants';
import type { GpuiSidebarRuntime } from './core';
import {
  formatGpuiCloseAfterDoneCountdown,
  formatGpuiDelayedSendDelay,
  isGpuiCloseAfterDonePresentationSessionDone,
  writeStoredGpuiCloseAfterDoneSessionIds,
} from './helpers/close-after-done';
import { parseGpuiRemotePresentationSessionId } from './helpers/remote-presentation';
import type { GpuiCloseAfterDoneTimer } from './types-and-protocol';
import type {
  GxserverPresentationCloseAfterDoneProjection,
  GxserverPresentationDelayedSendProjection,
} from '@/packages/shared/gxserver-presentation-sidebar-projection';
import { parseGxserverPresentationProjectSessionId } from '@/packages/shared/gxserver-presentation-sidebar-projection';
import type { GxserverPresentationSession } from '@/packages/shared/gxserver-protocol';
import type { SidebarToExtensionMessage } from '@/packages/shared/session-grid-contract';

/*
CDXC:RepoStructure 2026-08-22:
The method signatures below are copied verbatim from the original class body.
They exist as a standalone interface — rather than being derived from
`typeof gpuiSidebarRuntimeCloseAfterDoneMethods` — because deriving them would make
`GpuiSidebarRuntime` depend on the bodies that depend on it, which TypeScript
reports as a circular base type. `gpuiSidebarRuntimeCloseAfterDoneMethodsShapeCheck`
at the bottom of this file is what keeps the two in step.
*/
export interface GpuiSidebarRuntimeCloseAfterDoneMethods {
  scheduleRemoteDelayedSend(
    message: Extract<SidebarToExtensionMessage, { type: 'scheduleDelayedSend' }>
  ): Promise<void>;
  postponeDelayedSend(sessionId: string, delayMs: number): Promise<void>;
  cancelDelayedSend(sessionId: string): Promise<void>;
  toggleCloseAfterDone(sessionId: string): void;
  findPresentationSessionRowForSidebarSessionId(sessionId: string): GxserverPresentationSession | undefined;
  refreshCloseAfterDoneTimers(): void;
  refreshCloseAfterDoneTimer(sessionId: string, nowMs: number): void;
  resetCloseAfterDoneCountdown(sessionId: string, timer: GpuiCloseAfterDoneTimer): void;
  completeCloseAfterDoneTimer(sessionId: string, expectedDeadlineAtMs: number): void;
  clearCloseAfterDoneTimer(sessionId: string): void;
  persistCloseAfterDoneSessionIds(): void;
  ensureCloseAfterDoneCountdownTicker(): void;
  stopCloseAfterDoneCountdownTickerIfIdle(): void;
  hasActiveCloseAfterDoneCountdown(): boolean;
  getCloseAfterDoneProjection(sessionId: string): GxserverPresentationCloseAfterDoneProjection | undefined;
  getDelayedSendProjection(sessionId: string): GxserverPresentationDelayedSendProjection | undefined;
}

export const gpuiSidebarRuntimeCloseAfterDoneMethods = {
  async scheduleRemoteDelayedSend(
    this: GpuiSidebarRuntime,
    message: Extract<SidebarToExtensionMessage, { type: 'scheduleDelayedSend' }>
  ): Promise<void> {
    /*
    CDXC:DelayedSend 2026-08-17:
    Remote delayed sends are owned by the gxserver hosting the target session.
    The renderer submits only the canonical trigger and ids; the daemon stores,
    projects, and eventually fires the send even if this app disappears.
    */
    const reference = parseGpuiRemotePresentationSessionId(message.sessionId);
    if (!reference) {
      this.handleUnsupportedSidebarMessage(message);
      return;
    }
    const session = this.findRemotePresentationSession(reference);
    if (!session || (session.kind !== 'terminal' && session.kind !== 'agent')) {
      this.postSidebarActionToast('info', 'Delayed Send is only available for remote terminal sessions.');
      return;
    }

    const trigger = message.sendWhenSpecificAgentFinishes
      ? 'specificAgentStops'
      : message.sendWhenAllProjectSessionsStop
        ? 'allAgentsStop'
        : message.sendWhenAgentStops
          ? 'agentStops'
          : 'afterDelay';
    let delayMs: number | undefined;
    let description: string;
    if (trigger === 'specificAgentStops') {
      description = 'Presses Enter after the selected agent has finished working for 10 seconds.';
    } else if (trigger === 'allAgentsStop') {
      description = 'Presses Enter after all agents in the project have finished working for 10 seconds.';
    } else if (trigger === 'agentStops') {
      description = 'Presses Enter after this agent has finished working for 10 seconds.';
    } else {
      delayMs = message.delayMs;
      if (
        delayMs === undefined ||
        !Number.isSafeInteger(delayMs) ||
        delayMs <= 0 ||
        delayMs > GPUI_DELAYED_SEND_MAX_DELAY_MS
      ) {
        this.postSidebarActionToast('warning', 'Choose a future send time within 24 days.');
        return;
      }
      description = `Presses Enter in ${formatGpuiDelayedSendDelay(delayMs)}.`;
    }

    try {
      await this.requestRemoteGxserver(reference.machineId, '/api/scheduleDelayedSend', {
        ...(delayMs === undefined ? {} : { delayMs }),
        projectId: reference.projectId,
        ...(trigger === 'allAgentsStop' ? { sendWhenAllProjectSessionsStop: true } : {}),
        ...(trigger === 'agentStops' ? { sendWhenAgentStops: true } : {}),
        ...(trigger === 'specificAgentStops'
          ? { sendWhenSpecificAgentFinishes: message.sendWhenSpecificAgentFinishes }
          : {}),
        sessionId: reference.sessionId,
      });
      this.postSidebarActionToast('info', 'Delayed Send scheduled', { description });
    } catch (error) {
      this.postRemoteToast('error', 'Delayed Send unavailable', {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  },

  async postponeDelayedSend(this: GpuiSidebarRuntime, sessionId: string, delayMs: number): Promise<void> {
    /**
     * CDXC:DelayedSend 2026-09-15 WHY:
     * The card posts directly to this runtime, unlike the native scheduling modal. A remote-only handler silently dropped local postponements, leaving both the saved deadline and sidebar countdown unchanged.
     */
    const remote = parseGpuiRemotePresentationSessionId(sessionId);
    const reference = remote ?? parseGxserverPresentationProjectSessionId(sessionId);
    try {
      if (!reference) {
        throw new Error('The selected agent session is unavailable.');
      }
      const params = {
        projectId: reference.projectId,
        sessionId: reference.sessionId,
        delayMs,
      };
      if (remote) {
        await this.requestRemoteGxserver(remote.machineId, '/api/postponeDelayedSend', params);
      } else {
        if (!this.client) {
          throw new Error('The local gxserver is disconnected.');
        }
        await this.client.rpc('/api/postponeDelayedSend', params);
      }
      this.postSidebarActionToast('info', 'Delayed Send postponed', {
        description: `Added ${formatGpuiDelayedSendDelay(delayMs)} to the scheduled send time.`,
      });
    } catch (error) {
      this.postSidebarActionToast('error', 'Delayed Send could not be postponed', {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  },

  async cancelDelayedSend(this: GpuiSidebarRuntime, sessionId: string): Promise<void> {
    const remote = parseGpuiRemotePresentationSessionId(sessionId);
    const reference = remote ?? parseGxserverPresentationProjectSessionId(sessionId);
    try {
      if (!reference) {
        throw new Error('The selected agent session is unavailable.');
      }
      const params = { projectId: reference.projectId, sessionId: reference.sessionId };
      let result: { changed?: boolean };
      if (remote) {
        result = await this.requestRemoteGxserver(remote.machineId, '/api/cancelDelayedSend', params);
      } else {
        if (!this.client) {
          throw new Error('The local gxserver is disconnected.');
        }
        result = await this.client.rpc('/api/cancelDelayedSend', params);
      }
      this.postSidebarActionToast(
        'info',
        result.changed === true ? 'Delayed Send canceled' : 'No Delayed Send timer is active'
      );
    } catch (error) {
      this.postSidebarActionToast('error', 'Delayed Send could not be canceled', {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  },

  toggleCloseAfterDone(this: GpuiSidebarRuntime, sessionId: string): void {
    const session = this.findPresentationSessionRowForSidebarSessionId(sessionId);
    if (!session) {
      this.postSidebarActionToast('info', 'Close After Done is only available for terminal sessions.');
      return;
    }
    if (this.closeAfterDoneTimersBySessionId.has(sessionId)) {
      this.clearCloseAfterDoneTimer(sessionId);
      this.publishPresentation('patch');
      this.postSidebarActionToast('info', 'Close After Done canceled');
      return;
    }
    this.closeAfterDoneTimersBySessionId.set(sessionId, {});
    this.persistCloseAfterDoneSessionIds();
    this.refreshCloseAfterDoneTimer(sessionId, Date.now());
    this.publishPresentation('patch');
    this.postSidebarActionToast('info', 'Close After Done enabled', {
      description: 'Closes after Done stays visible for 3m.',
    });
  },

  findPresentationSessionRowForSidebarSessionId(
    this: GpuiSidebarRuntime,
    sessionId: string
  ): GxserverPresentationSession | undefined {
    const remoteSession = parseGpuiRemotePresentationSessionId(sessionId);
    if (remoteSession) {
      return this.findRemotePresentationSession(remoteSession);
    }
    const reference = parseGxserverPresentationProjectSessionId(sessionId);
    if (!reference) {
      return undefined;
    }
    return this.presentation?.sessions.find(
      (session) => session.projectId === reference.projectId && session.sessionId === reference.sessionId
    );
  },

  refreshCloseAfterDoneTimers(this: GpuiSidebarRuntime): void {
    const nowMs = Date.now();
    for (const sessionId of [...this.closeAfterDoneTimersBySessionId.keys()]) {
      this.refreshCloseAfterDoneTimer(sessionId, nowMs);
    }
  },

  refreshCloseAfterDoneTimer(this: GpuiSidebarRuntime, sessionId: string, nowMs: number): void {
    const timer = this.closeAfterDoneTimersBySessionId.get(sessionId);
    if (!timer) {
      return;
    }
    const remoteSession = parseGpuiRemotePresentationSessionId(sessionId);
    const snapshotAvailable = remoteSession
      ? this.remotePresentations.has(remoteSession.machineId)
      : this.presentation !== undefined;
    if (!snapshotAvailable) {
      this.resetCloseAfterDoneCountdown(sessionId, timer);
      return;
    }
    const session = this.findPresentationSessionRowForSidebarSessionId(sessionId);
    if (!session) {
      this.clearCloseAfterDoneTimer(sessionId);
      return;
    }
    if (!isGpuiCloseAfterDonePresentationSessionDone(session)) {
      this.resetCloseAfterDoneCountdown(sessionId, timer);
      return;
    }
    if (timer.deadlineAtMs !== undefined) {
      this.ensureCloseAfterDoneCountdownTicker();
      return;
    }
    const deadlineAtMs = nowMs + GPUI_CLOSE_AFTER_DONE_DELAY_MS;
    const timeoutId = window.setTimeout(() => {
      this.completeCloseAfterDoneTimer(sessionId, deadlineAtMs);
    }, GPUI_CLOSE_AFTER_DONE_DELAY_MS);
    this.closeAfterDoneTimersBySessionId.set(sessionId, {
      deadlineAtMs,
      doneSinceAtMs: nowMs,
      timeoutId,
    });
    this.ensureCloseAfterDoneCountdownTicker();
  },

  resetCloseAfterDoneCountdown(this: GpuiSidebarRuntime, sessionId: string, timer: GpuiCloseAfterDoneTimer): void {
    if (timer.timeoutId !== undefined) {
      window.clearTimeout(timer.timeoutId);
    }
    this.closeAfterDoneTimersBySessionId.set(sessionId, {});
    this.stopCloseAfterDoneCountdownTickerIfIdle();
  },

  completeCloseAfterDoneTimer(this: GpuiSidebarRuntime, sessionId: string, expectedDeadlineAtMs: number): void {
    const timer = this.closeAfterDoneTimersBySessionId.get(sessionId);
    if (!timer || timer.deadlineAtMs !== expectedDeadlineAtMs) {
      return;
    }
    const session = this.findPresentationSessionRowForSidebarSessionId(sessionId);
    if (!session || !isGpuiCloseAfterDonePresentationSessionDone(session)) {
      this.resetCloseAfterDoneCountdown(sessionId, timer);
      this.publishPresentation('patch');
      return;
    }
    this.clearCloseAfterDoneTimer(sessionId);
    void this.transitionSession(sessionId, 'close');
  },

  clearCloseAfterDoneTimer(this: GpuiSidebarRuntime, sessionId: string): void {
    const timer = this.closeAfterDoneTimersBySessionId.get(sessionId);
    if (timer?.timeoutId !== undefined) {
      window.clearTimeout(timer.timeoutId);
    }
    this.closeAfterDoneTimersBySessionId.delete(sessionId);
    this.persistCloseAfterDoneSessionIds();
    this.stopCloseAfterDoneCountdownTickerIfIdle();
  },

  persistCloseAfterDoneSessionIds(this: GpuiSidebarRuntime): void {
    writeStoredGpuiCloseAfterDoneSessionIds([...this.closeAfterDoneTimersBySessionId.keys()]);
  },

  ensureCloseAfterDoneCountdownTicker(this: GpuiSidebarRuntime): void {
    if (this.closeAfterDoneCountdownTickerId !== undefined) {
      return;
    }
    this.closeAfterDoneCountdownTickerId = window.setInterval(() => {
      if (!this.hasActiveCloseAfterDoneCountdown()) {
        this.stopCloseAfterDoneCountdownTickerIfIdle();
        return;
      }
      this.publishPresentation('patch');
    }, 1_000);
  },

  stopCloseAfterDoneCountdownTickerIfIdle(this: GpuiSidebarRuntime): void {
    if (this.hasActiveCloseAfterDoneCountdown() || this.closeAfterDoneCountdownTickerId === undefined) {
      return;
    }
    window.clearInterval(this.closeAfterDoneCountdownTickerId);
    this.closeAfterDoneCountdownTickerId = undefined;
  },

  hasActiveCloseAfterDoneCountdown(this: GpuiSidebarRuntime): boolean {
    for (const timer of this.closeAfterDoneTimersBySessionId.values()) {
      if (timer.deadlineAtMs !== undefined) {
        return true;
      }
    }
    return false;
  },

  getCloseAfterDoneProjection(
    this: GpuiSidebarRuntime,
    sessionId: string
  ): GxserverPresentationCloseAfterDoneProjection | undefined {
    const timer = this.closeAfterDoneTimersBySessionId.get(sessionId);
    if (!timer) {
      return undefined;
    }
    if (timer.deadlineAtMs === undefined) {
      return { armed: true };
    }
    const remainingMs = Math.max(0, timer.deadlineAtMs - Date.now());
    return {
      armed: true,
      deadlineAt: new Date(timer.deadlineAtMs).toISOString(),
      remainingLabel: formatGpuiCloseAfterDoneCountdown(remainingMs),
      remainingMs,
    };
  },

  getDelayedSendProjection(
    this: GpuiSidebarRuntime,
    sessionId: string
  ): GxserverPresentationDelayedSendProjection | undefined {
    const delayedSend = this.workspaceSessionDelayedSends.get(sessionId);
    if (!delayedSend) {
      return undefined;
    }
    return {
      deadlineAt: delayedSend.delayedSendDeadlineAt,
      remainingLabel: delayedSend.delayedSendRemainingLabel,
      remainingMs: delayedSend.delayedSendRemainingMs,
      sendWhenAllProjectSessionsStopActive:
        delayedSend.sendWhenAllProjectSessionsStopActive === true ? true : undefined,
      sendWhenAgentStopsActive: delayedSend.sendWhenAgentStopsActive === true ? true : undefined,
    };
  },
};

const gpuiSidebarRuntimeCloseAfterDoneMethodsShapeCheck: GpuiSidebarRuntimeCloseAfterDoneMethods =
  gpuiSidebarRuntimeCloseAfterDoneMethods;
void gpuiSidebarRuntimeCloseAfterDoneMethodsShapeCheck;
