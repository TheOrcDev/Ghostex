import { arrayCodec, enumCodec, objectCodec, stringListCodec, textCodec } from './codecs';
import { KiB, MiB } from './budgets';
import type { StorageCodec, StoreDefinition, StorageBackend, StoragePolicy } from './types';

const day = 86_400_000;
function define<T>(
  id: string,
  owner: string,
  source: string,
  key: string,
  codec: StorageCodec<T>,
  options: Partial<Omit<StoreDefinition<T>, 'id' | 'owner' | 'source' | 'key' | 'codec'>> = {}
): StoreDefinition<T> {
  return Object.freeze({
    id,
    owner,
    source,
    key,
    codec,
    version: 1,
    backend: 'local',
    policy: 'preference',
    collection: false,
    maxEntryBytes: 64 * KiB,
    maxBytes: 128 * KiB,
    maxEntries: 1,
    maxAgeMs: null,
    ...options,
  });
}
const collection = { collection: true, maxEntries: 2_000 };
const disk = { ...collection, backend: 'indexeddb' as StorageBackend, maxEntryBytes: 2 * MiB, maxBytes: 16 * MiB };
const protectedDisk = { ...disk, policy: 'protected' as StoragePolicy, maxEntries: 50_000, maxBytes: 32 * MiB };
const cache = { ...disk, policy: 'cache' as StoragePolicy, maxAgeMs: 30 * day };
const binary = enumCodec(['1', '0']);
const boolean = enumCodec(['true', 'false']);
const core = 'packages/core-ui/';
const chat = core + 'chat/';
const desktop = 'apps/desktop/';

/**
 * CDXC:Settings 2026-09-15 DECISION:
 * User: all browser storage must go through one system so storage cannot silently fill up and we can identify what owns the space.
 * Every namespace declares its owner, schema, budget and retention here; protected user work has no automatic eviction.
 */
export const storageCatalog = Object.freeze({
  addRepository: define(
    'addRepository',
    'Add repository',
    core + 'add-repository-modal.tsx',
    'ghostex.addRepository.lastLocation',
    textCodec
  ),
  exportOptions: define(
    'exportOptions',
    'Transcript export',
    core + 'export-transcript-result-modal.tsx',
    'ghostex.exportTranscript.includeOptions',
    objectCodec
  ),
  exportMode: define(
    'exportMode',
    'Transcript export',
    core + 'export-transcript-result-modal.tsx',
    'ghostex.exportTranscript.mode',
    enumCodec(['export', 'handoff'])
  ),
  gitDiff: define(
    'gitDiff',
    'Git diff preferences',
    core + 'git-commit-modal.tsx',
    'ghostex.gitCommitModal.diffPreferences.v1',
    objectCodec
  ),
  themeHistory: define(
    'themeHistory',
    'Recent project colors',
    core + 'workspace-theme-color-history.ts',
    'ghostex-workspace-theme-color-history',
    stringListCodec
  ),
  hiddenItems: define(
    'hiddenItems',
    'Sidebar visibility',
    core + 'sidebar-hidden-items.ts',
    'ghostex.sidebar.hidden-items.v1',
    objectCodec
  ),
  collections: define(
    'collections',
    'Project collections',
    core + 'project-collections.ts',
    'ghostex.sidebar.projectCollections.v1',
    objectCodec,
    { maxEntryBytes: 256 * KiB, maxBytes: 256 * KiB }
  ),
  launcher: define(
    'launcher',
    'Agent launcher',
    core + 'primary-agent-launcher.ts',
    'ghostex-sidebar-project-terminal-launcher',
    textCodec
  ),
  accountOwner: define(
    'accountOwner',
    'Account setup identity',
    core + 'accounts/setup-monitor.ts',
    'ghostex.accountSetupOwner',
    textCodec,
    { policy: 'protected' }
  ),
  importIntro: define(
    'importIntro',
    'Import introduction',
    core + 'sidebar-app/import-sessions-card.tsx',
    'ghostex.sidebar.import-sessions-intro-seen.v1',
    boolean
  ),
  machineTab: define(
    'machineTab',
    'Selected machine',
    core + 'sidebar-app/machine-tab-selection.ts',
    'ghostex-sidebar-selected-machine-tab',
    textCodec,
    { ...collection }
  ),
  collapse: define(
    'collapse',
    'Sidebar disclosure',
    core + 'sidebar-app/collapse-state.ts',
    'ghostex-sidebar-ui-collapse-state',
    objectCodec,
    { ...collection, maxBytes: 256 * KiB }
  ),
  codeWrap: define(
    'codeWrap',
    'Chat code wrapping',
    chat + 'session-chat-code-wrap.ts',
    'ghostex.sessionChat.codeWrap',
    binary
  ),
  verbose: define(
    'verbose',
    'Chat verbose preference',
    chat + 'session-chat-verbose-override.ts',
    'ghostex.sessionChat.verbose.',
    binary,
    disk
  ),
  summary: define(
    'summary',
    'Chat summary preference',
    chat + 'session-chat-summary-override.ts',
    'ghostex.sessionChat.summary.',
    binary,
    disk
  ),
  tasksCollapsed: define(
    'tasksCollapsed',
    'Chat task panel',
    chat + 'session-chat-agent-tasks-panel.tsx',
    'ghostex.chat.agentTasks.collapsed',
    binary
  ),
  terminalExpanded: define(
    'terminalExpanded',
    'Chat terminal output',
    chat + 'session-chat-terminal-tool-row.tsx',
    'ghostex.sessionChat.terminalToolExpanded',
    boolean
  ),
  claudeContext: define(
    'claudeContext',
    'Claude context display',
    chat + 'session-chat-context-details.ts',
    'ghostex.chat.context-details.v1',
    objectCodec
  ),
  codexContext: define(
    'codexContext',
    'Codex context display',
    chat + 'session-chat-context-details.ts',
    'ghostex.chat.context-details.codex.v1',
    objectCodec
  ),
  notices: define(
    'notices',
    'Dismissed chat notices',
    chat + 'session-chat-terminal-notice-card.tsx',
    'ghostex.sessionChat.noticeDismissed.',
    textCodec,
    disk
  ),
  sessionOptions: define(
    'sessionOptions',
    'Session model options',
    chat + 'session-chat-session-options.ts',
    'ghostex.sessionChat.options.',
    objectCodec,
    cache
  ),
  modelOutbox: define(
    'modelOutbox',
    'Pending model selections',
    chat + 'session-chat-model-picker-launcher.tsx',
    'ghostex.model-selection-outbox.',
    objectCodec,
    protectedDisk
  ),
  retiredQuestions: define(
    'retiredQuestions',
    'Answered question receipts',
    chat + 'session-chat-async-questions.tsx',
    'ghostex:async-questions:',
    stringListCodec,
    protectedDisk
  ),
  questionDrafts: define(
    'questionDrafts',
    'Unsent question answers',
    chat + 'session-chat-question-drafts.ts',
    'ghostex.sessionChat.questionDraft.',
    objectCodec,
    protectedDisk
  ),
  interactions: define(
    'interactions',
    'Chat display state',
    chat + 'session-chat-interaction-state.tsx',
    'ghostex.session-chat.interactions.v1:',
    objectCodec,
    { ...cache, maxEntries: 100 }
  ),
  composerSelection: define(
    'composerSelection',
    'Parked composer selection',
    chat + 'session-chat-composer-parking.ts',
    'ghostex.sessionChat.composerSelection.',
    objectCodec,
    protectedDisk
  ),
  chatClient: define(
    'chatClient',
    'Chat client identity',
    chat + 'session-chat-queue.ts',
    'ghostex.sessionChat.clientId',
    textCodec,
    { policy: 'protected' }
  ),
  returnedPrompts: define(
    'returnedPrompts',
    'Returned prompt receipts',
    chat + 'session-chat-returned-prompt.ts',
    'ghostex.sessionChat.returnedPrompts.applied',
    stringListCodec,
    { policy: 'protected' }
  ),
  sentHistory: define(
    'sentHistory',
    'Sent prompt history',
    chat + 'session-chat-sent-history.ts',
    'ghostex.sessionChat.sent.',
    objectCodec,
    { ...cache, maxEntries: 50, maxAgeMs: null, retainedAt: (raw) => Date.parse(JSON.parse(raw).createdAt) }
  ),
  deliveryReceipts: define(
    'deliveryReceipts',
    'Prompt delivery receipts',
    chat + 'session-chat-sent-history.ts',
    'ghostex.sessionChat.delivered.',
    stringListCodec,
    protectedDisk
  ),
  drafts: define(
    'drafts',
    'Unsent chat drafts',
    chat + 'session-chat-draft-storage.ts',
    'ghostex.sessionChat.draft.',
    textCodec,
    protectedDisk
  ),
  recovery: define(
    'recovery',
    'Draft recovery checkpoints',
    chat + 'session-chat-draft-recovery.ts',
    'ghostex.sessionChat.recovery.',
    textCodec,
    protectedDisk
  ),
  recoveryDismissed: define(
    'recoveryDismissed',
    'Draft dismissal receipts',
    chat + 'session-chat-draft-dismissals.ts',
    'ghostex.sessionChat.recoveryDismissed.',
    arrayCodec,
    protectedDisk
  ),
  draftOutbox: define(
    'draftOutbox',
    'Pending draft revisions',
    chat + 'session-chat-draft-outbox.ts',
    'ghostex.sessionChat.outbox.',
    objectCodec,
    protectedDisk
  ),
  chatSnapshots: define(
    'chatSnapshots',
    'Recent conversation cache',
    desktop + 'sidebar/session-chat-runtime/persistence.ts',
    'ghostex.sessionChat.snapshot.',
    objectCodec,
    {
      ...cache,
      maxEntries: 24,
      maxBytes: 32 * MiB,
      maxAgeMs: 7 * day,
      retainedAt: (raw) => Number(JSON.parse(raw).savedAt),
    }
  ),
  workspaceGroups: define(
    'workspaceGroups',
    'Workspace session groups',
    desktop + 'sidebar/workspace-session-groups.ts',
    'ghostex-gpui-workspace-session-groups',
    objectCodec,
    { maxEntryBytes: 256 * KiB, maxBytes: 256 * KiB }
  ),
  projectLastSession: define(
    'projectLastSession',
    'Last session per project',
    desktop + 'sidebar/gxserver-runtime/project-activation.ts',
    'ghostex.gpui.project-last-session.v1:',
    textCodec,
    disk
  ),
  closeAfterDone: define(
    'closeAfterDone',
    'Close after done',
    desktop + 'sidebar/gxserver-runtime/helpers/close-after-done.ts',
    'ghostex-gpui-close-after-done-session-ids',
    stringListCodec
  ),
  remoteOrder: define(
    'remoteOrder',
    'Remote project ordering',
    desktop + 'sidebar/gxserver-runtime/helpers/recent-projects.ts',
    'ghostex-gpui-remote-group-order',
    objectCodec
  ),
  remoteRecents: define(
    'remoteRecents',
    'Remote recent projects',
    desktop + 'sidebar/gxserver-runtime/helpers/recent-projects.ts',
    'ghostex-gpui-remote-recent-projects',
    arrayCodec
  ),
  remotePresentations: define(
    'remotePresentations',
    'Remote presentation cache',
    desktop + 'sidebar/gxserver-runtime/helpers/remote-last-seen.ts',
    'ghostex-gpui-remote-last-seen-presentations',
    objectCodec,
    { ...cache, maxEntryBytes: 8 * MiB, maxBytes: 24 * MiB, maxEntries: 32 }
  ),
  nativeSettings: define(
    'nativeSettings',
    'Legacy native settings cache',
    desktop + 'views/project-board/constants.ts',
    'ghostex-native-settings',
    objectCodec,
    { policy: 'cache', maxAgeMs: null }
  ),
  boardView: define(
    'boardView',
    'Board view preferences',
    desktop + 'views/project-board/constants.ts',
    'ghostex-project-board-view',
    objectCodec
  ),
  boardCards: define(
    'boardCards',
    'Board card preferences',
    desktop + 'views/project-board/card-view-options.ts',
    'ghostexProjectBoardCardView.v1',
    objectCodec
  ),
  docsSide: define(
    'docsSide',
    'Docs sidebar position',
    desktop + 'views/manage/manage-app.tsx',
    'ghostex.manage.sidebarSide',
    enumCodec(['left', 'right'])
  ),
  docsWidth: define(
    'docsWidth',
    'Docs sidebar width',
    desktop + 'views/manage/manage-app.tsx',
    'ghostex.manage.sidebarWidth',
    textCodec
  ),
  docsPinned: define(
    'docsPinned',
    'Docs sidebar pin',
    desktop + 'views/manage/manage-app.tsx',
    'ghostex.manage.sidebarPinned',
    boolean
  ),
  docsFormatting: define(
    'docsFormatting',
    'Docs formatting bar',
    desktop + 'views/manage/meo-toolbar.tsx',
    'ghostex.manage.formattingBarCollapsed',
    boolean
  ),
  docsIndex: define(
    'docsIndex',
    'Docs directory cache',
    desktop + 'views/manage/file-index.ts',
    'ghostex-docs-index-v1:',
    objectCodec,
    { ...cache, backend: 'session', maxEntryBytes: 512 * KiB, maxBytes: MiB, maxEntries: 2 }
  ),
  docsOpenFiles: define(
    'docsOpenFiles',
    'Open documents',
    desktop + 'views/manage/open-documents.ts',
    'ghostex.manage.openFiles.',
    stringListCodec,
    disk
  ),
  docsDrafts: define(
    'docsDrafts',
    'Unsaved documents',
    desktop + 'views/manage/open-documents.ts',
    'ghostex.manage.drafts.',
    objectCodec,
    { ...protectedDisk, maxEntryBytes: 8 * MiB }
  ),
  docsActiveFile: define(
    'docsActiveFile',
    'Selected document',
    desktop + 'views/manage/manage-app.tsx',
    'ghostex.manage.activeFile.',
    textCodec,
    disk
  ),
  commitAgent: define(
    'commitAgent',
    'Commit prompt agent',
    desktop + 'views/modal-host.tsx',
    'ghostex.promptAgent.gitCommit',
    textCodec
  ),
  renameAgent: define(
    'renameAgent',
    'Rename prompt agent',
    desktop + 'views/modal-host.tsx',
    'ghostex.promptAgent.renameSession',
    textCodec
  ),
  terminalModel: define(
    'terminalModel',
    'Terminal model selection',
    desktop + 'views/model-picker-host.tsx',
    'ghostex.terminal-model-picker.',
    objectCodec,
    cache
  ),
  openTarget: define(
    'openTarget',
    'Open in target',
    desktop + 'views/titlebar/settings-io.ts',
    'ghostex.titlebar.lastOpenTargetId',
    textCodec
  ),
  lastAction: define(
    'lastAction',
    'Last project command',
    desktop + 'views/titlebar/settings-io.ts',
    'ghostex.titlebar.lastActionCommandByProject:',
    textCodec,
    disk
  ),
  keepAwake: define(
    'keepAwake',
    'Keep Awake runtime',
    desktop + 'views/titlebar/project-state.ts',
    'ghostex.titlebar.keepAwakeRuntime',
    objectCodec
  ),
  keepAwakeSync: define(
    'keepAwakeSync',
    'Keep Awake synchronization',
    desktop + 'views/titlebar/project-state.ts',
    'ghostex.titlebar.keepAwakeRuntimeSync',
    objectCodec
  ),
  lidSleep: define(
    'lidSleep',
    'Lid sleep prevention',
    desktop + 'views/titlebar/app.tsx',
    'ghostex.titlebar.lidSleepPrevention',
    enumCodec(['enabled', 'disabled'])
  ),
  titlebarGit: define(
    'titlebarGit',
    'Titlebar Git cache',
    desktop + 'views/titlebar/project-state.ts',
    'ghostex.titlebar.gitState.',
    objectCodec,
    cache
  ),
  tipsRead: define(
    'tipsRead',
    'Read tips',
    desktop + 'views/titlebar/project-state.ts',
    'ghostex.titlebar.tips.readIds',
    stringListCodec
  ),
  modelCatalog: define(
    'modelCatalog',
    'Agent model catalog',
    'packages/shared/agent-model-catalog-store.ts',
    'ghostex.agentModelCatalog.v1',
    objectCodec,
    { ...cache, collection: false, maxEntries: 1 }
  ),
  webSettings: define(
    'webSettings',
    'Web settings',
    'apps/web/src/app/web-settings.ts',
    'ghostexWeb.settings.v1',
    objectCodec,
    { maxEntryBytes: 256 * KiB, maxBytes: 256 * KiB }
  ),
  webSidebarWidth: define(
    'webSidebarWidth',
    'Web sidebar width',
    'apps/web/src/routes/__root.tsx',
    'ghostexWeb.sidebarWidth.v1',
    textCodec
  ),
  webSidebarCollapsed: define(
    'webSidebarCollapsed',
    'Web sidebar disclosure',
    'apps/web/src/routes/__root.tsx',
    'ghostexWeb.sidebarCollapsed.v1',
    boolean
  ),
  webWorkspace: define(
    'webWorkspace',
    'Web workspace',
    'apps/web/src/workspace/workspace-storage.ts',
    'ghostexWeb.workspace.v1',
    objectCodec,
    { maxEntryBytes: 256 * KiB, maxBytes: 256 * KiB }
  ),
  webMachines: define(
    'webMachines',
    'Web machine connections',
    'apps/web/src/machines/machine-catalog.ts',
    'ghostexWeb.machines.v1',
    arrayCodec,
    { policy: 'protected' }
  ),
  webCommandHeight: define(
    'webCommandHeight',
    'Web command pane height',
    'apps/web/src/app/command-pane.tsx',
    'ghostexWeb.commandPaneHeight.v1',
    textCodec
  ),
  webDebugConnections: define(
    'webDebugConnections',
    'Web connection debugging',
    'apps/web/src/connections/gxserver-connection.ts',
    'ghostexWeb.debugConnections',
    binary
  ),
  webDebugSidebar: define(
    'webDebugSidebar',
    'Web sidebar debugging',
    'apps/web/src/sidebar-runtime/sidebar-runtime.ts',
    'ghostexWeb.debugSidebar',
    binary
  ),
} as const);

export type StoreId = keyof typeof storageCatalog;
export const definitions: readonly StoreDefinition[] = Object.values(storageCatalog);
export function owns(definition: StoreDefinition, key: string): boolean {
  return definition.collection ? key.startsWith(definition.key) : key === definition.key;
}
export function definitionForKey(key: string): StoreDefinition | undefined {
  return definitions.find((definition) => owns(definition, key));
}
