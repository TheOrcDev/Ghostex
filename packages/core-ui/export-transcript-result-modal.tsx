import { storageScope } from '@/packages/client-storage';
import {
  IconCheck,
  IconCircleCheckFilled,
  IconCopy,
  IconFolderSearch,
  IconLoader2,
  IconMarkdown,
  IconUserShare,
} from '@tabler/icons-react';
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from 'react';
import { Button } from '@/packages/components/ui/button';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/packages/components/ui/select';
import { Switch } from '@/packages/components/ui/switch';
import { type SidebarAgentButton } from '../shared/sidebar-agents';
import {
  APP_MODAL_SELECT_CONTENT_CLASS,
  AppModalButton,
  AppModalDescription,
  AppModalFooter,
  AppModalHeader,
  AppModalShell,
  AppModalTitle,
} from './app-modal-shell';
import { AppTooltip } from './app-tooltip';
import { playCopySound } from './copy-sound';

const clientStorage = storageScope(["exportOptions","exportMode"]);

/**
 * CDXC:TranscriptExport 2026-08-24:
 * The export dialog's include-toggles. User and agent messages are never
 * optional, so only the three optional record families appear here. The
 * defaults mirror the daemon's historical selection: commands and patches in,
 * reasoning out. The user's last combination is a per-client UI preference so
 * repeat exports reopen exactly as they left them without involving gxserver.
 */
export type ExportTranscriptIncludeOptions = {
  includeCommands: boolean;
  includePatches: boolean;
  includeReasoning: boolean;
};

export const DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS: ExportTranscriptIncludeOptions = {
  includeCommands: true,
  includePatches: true,
  includeReasoning: false,
};

/** What the user wants to do with the written file. */
export type ExportTranscriptMode = 'handoff' | 'export';

const EXPORT_TRANSCRIPT_INCLUDE_OPTIONS_STORAGE_KEY = 'ghostex.exportTranscript.includeOptions';
const EXPORT_TRANSCRIPT_MODE_STORAGE_KEY = 'ghostex.exportTranscript.mode';

function readExportTranscriptIncludeOptions(): ExportTranscriptIncludeOptions {
  if (typeof window === 'undefined') {
    return DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS;
  }
  try {
    const stored = JSON.parse(
      clientStorage.getItem(EXPORT_TRANSCRIPT_INCLUDE_OPTIONS_STORAGE_KEY) ?? 'null'
    ) as Partial<ExportTranscriptIncludeOptions> | null;
    return {
      includeCommands:
        typeof stored?.includeCommands === 'boolean'
          ? stored.includeCommands
          : DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS.includeCommands,
      includePatches:
        typeof stored?.includePatches === 'boolean'
          ? stored.includePatches
          : DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS.includePatches,
      includeReasoning:
        typeof stored?.includeReasoning === 'boolean'
          ? stored.includeReasoning
          : DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS.includeReasoning,
    };
  } catch {
    return DEFAULT_EXPORT_TRANSCRIPT_INCLUDE_OPTIONS;
  }
}

function writeExportTranscriptIncludeOptions(options: ExportTranscriptIncludeOptions): void {
  try {
    clientStorage.setItem(EXPORT_TRANSCRIPT_INCLUDE_OPTIONS_STORAGE_KEY, JSON.stringify(options));
  } catch {
    // Storage can be unavailable in isolated web, test, and story contexts.
  }
}

function readExportTranscriptMode(): ExportTranscriptMode {
  if (typeof window === 'undefined') {
    return 'handoff';
  }
  try {
    return clientStorage.getItem(EXPORT_TRANSCRIPT_MODE_STORAGE_KEY) === 'export' ? 'export' : 'handoff';
  } catch {
    return 'handoff';
  }
}

function writeExportTranscriptMode(mode: ExportTranscriptMode): void {
  try {
    clientStorage.setItem(EXPORT_TRANSCRIPT_MODE_STORAGE_KEY, mode);
  } catch {
    // Storage can be unavailable in isolated web, test, and story contexts.
  }
}

/**
 * The dialog's lifecycle, owned by the host: choose what to include, watch the
 * daemon write the file, then follow up on the result. `failed` keeps the
 * dialog open with the daemon's structured message (unsupported agent, no
 * transcript yet, …) and a way back to the export.
 */
export type ExportTranscriptModalStage =
  | { stage: 'options' }
  | { stage: 'exporting' }
  | { agentId?: string; canReveal: boolean; path: string; stage: 'done' }
  | { message: string; stage: 'failed' };

export type ExportTranscriptModalProps = {
  /** A follow-up action's failure (copy, session create), shown without leaving the done stage. */
  actionErrorMessage?: string;
  /** Configured agents offered for the follow-up conversation. */
  agents?: SidebarAgentButton[];
  /**
   * The exported session's own agent, preselected so the obvious "handoff to
   * the same agent" choice is one click away.
   */
  defaultAgentId?: string;
  /** Overrides the remembered mode on open; stories and tests use it to show one branch. */
  initialMode?: ExportTranscriptMode;
  isOpen: boolean;
  onClose: () => void;
  /** Runs the export with the chosen include-toggles. The host answers by moving `stage` forward. */
  onExport: (options: ExportTranscriptIncludeOptions) => void;
  onRevealInFinder?: () => void;
  onStartNewConversation: (agentId: string) => void;
  stage: ExportTranscriptModalStage;
  /** Disables Handoff while the host is creating the session. */
  startBusy?: boolean;
};

const INCLUDE_TOGGLE_ROWS: Array<{
  description: string;
  key: keyof ExportTranscriptIncludeOptions;
  label: string;
}> = [
  {
    description: 'Terminal commands the agent ran, with the tail of their output.',
    key: 'includeCommands',
    label: 'Commands & output',
  },
  {
    description: 'The patches the agent applied to files.',
    key: 'includePatches',
    label: 'File changes',
  },
  {
    description: "The agent's own reasoning sections.",
    key: 'includeReasoning',
    label: 'Reasoning',
  },
];

const EXPORT_TRANSCRIPT_PRIMARY_ACTION_ID = 'export-transcript-primary-action';

/**
 * CDXC:TranscriptExport 2026-09-15 DECISION:
 * User: Handoff / Export is one page, not an options page followed by a result page, and "Handoff to an agent" and
 * "Export to Markdown" are two explicit choices on it. The mode cards sit above the include-toggles; Handoff writes
 * the file and starts the follow-up conversation in one click, while Export swaps the toggles for the saved path
 * with Copy and Reveal. The native child window fits itself once on open, so the options layout is the tallest state
 * and the agent row keeps its height in Export mode instead of disappearing.
 * Starting a conversation never sends a prompt for the user: the mention is typed into the new agent's input and
 * left unsubmitted. Reveal is omitted, not disabled, when the file lives on another machine.
 */
export function ExportTranscriptModal({
  actionErrorMessage,
  agents = [],
  defaultAgentId,
  initialMode,
  isOpen,
  onClose,
  onExport,
  onRevealInFinder,
  onStartNewConversation,
  stage,
  startBusy = false,
}: ExportTranscriptModalProps) {
  const agentSelectId = useId();
  const [mode, setMode] = useState<ExportTranscriptMode>(() => initialMode ?? readExportTranscriptMode());
  const [selectedAgentId, setSelectedAgentId] = useState('');
  const [copied, setCopied] = useState(false);
  const [includeOptions, setIncludeOptions] = useState(readExportTranscriptIncludeOptions);
  const handoffRequestedRef = useRef(false);
  const promptAgents = useMemo(() => agents.filter((agent) => agent.command?.trim()), [agents]);
  const doneAgentId = stage.stage === 'done' ? stage.agentId : undefined;
  const effectiveAgentId =
    promptAgents.find((agent) => agent.agentId === selectedAgentId)?.agentId ??
    promptAgents.find((agent) => agent.agentId === (doneAgentId ?? defaultAgentId))?.agentId ??
    promptAgents[0]?.agentId ??
    '';
  const handoffAvailable = promptAgents.length > 0;
  const effectiveMode: ExportTranscriptMode = handoffAvailable ? mode : 'export';

  const isExporting = stage.stage === 'exporting';
  const isDone = stage.stage === 'done';
  const showResult = isDone && effectiveMode === 'export';
  const busy = isExporting || (isDone && effectiveMode === 'handoff') || startBusy;
  const canRun = !isDone && !isExporting && (effectiveMode === 'export' || Boolean(effectiveAgentId));

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    handoffRequestedRef.current = false;
    setMode(initialMode ?? readExportTranscriptMode());
    setSelectedAgentId('');
    setCopied(false);
    setIncludeOptions(readExportTranscriptIncludeOptions());
  }, [initialMode, isOpen]);

  useEffect(() => {
    if (!isDone || !handoffRequestedRef.current || !effectiveAgentId) {
      return;
    }
    handoffRequestedRef.current = false;
    onStartNewConversation(effectiveAgentId);
  }, [effectiveAgentId, isDone, onStartNewConversation]);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timeoutId = window.setTimeout(() => setCopied(false), 1_500);
    return () => window.clearTimeout(timeoutId);
  }, [copied]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      document.getElementById(EXPORT_TRANSCRIPT_PRIMARY_ACTION_ID)?.focus();
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [isOpen, stage.stage]);

  const run = () => {
    if (!canRun) {
      return;
    }
    handoffRequestedRef.current = effectiveMode === 'handoff';
    onExport(includeOptions);
  };

  const copyPath = () => {
    if (stage.stage !== 'done') {
      return;
    }
    playCopySound();
    void navigator.clipboard.writeText(stage.path).then(
      () => setCopied(true),
      () => setCopied(false)
    );
  };

  const chooseMode = (next: ExportTranscriptMode) => {
    setMode(next);
    writeExportTranscriptMode(next);
  };

  const onDialogKeyDownCapture = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key !== 'Enter' || event.repeat || event.nativeEvent.isComposing) {
      return;
    }
    const target = event.target;
    if (!(target instanceof HTMLElement)) {
      return;
    }
    if (target.closest('[role="listbox"]') || target.closest('[data-slot="select-content"]')) {
      return;
    }
    if (target.tagName === 'TEXTAREA' || target.tagName === 'INPUT' || target.tagName === 'BUTTON') {
      return;
    }
    if (!canRun) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    run();
  };

  const selectedAgentName = promptAgents.find((agent) => agent.agentId === effectiveAgentId)?.name;
  const primaryLabel = isExporting
    ? 'Exporting…'
    : startBusy || (isDone && effectiveMode === 'handoff')
      ? 'Starting…'
      : stage.stage === 'failed'
        ? 'Try Again'
        : effectiveMode === 'handoff'
          ? selectedAgentName
            ? `Handoff to ${selectedAgentName}`
            : 'Handoff'
          : 'Export';

  const modeCards: Array<{
    description: string;
    icon: typeof IconUserShare;
    mode: ExportTranscriptMode;
    title: string;
  }> = [
    ...(handoffAvailable
      ? [
          {
            description: 'Start a new conversation with the handover attached.',
            icon: IconUserShare,
            mode: 'handoff' as const,
            title: 'Handoff to an agent',
          },
        ]
      : []),
    {
      description: 'Save the conversation as a file and copy its path.',
      icon: IconMarkdown,
      mode: 'export' as const,
      title: 'Export to Markdown',
    },
  ];

  return (
    <AppModalShell
      className='export-transcript-modal-shadcn'
      isOpen={isOpen}
      onClose={onClose}
      onKeyDownCapture={onDialogKeyDownCapture}
      width={540}
    >
      <AppModalHeader className='gap-1'>
        <AppModalTitle>Handoff / Export</AppModalTitle>
        <AppModalDescription>
          Ghostex writes this conversation to a Markdown file. Pick what to do with it and what to include.
        </AppModalDescription>
      </AppModalHeader>
      <div className='export-transcript-modal-body'>
        <div aria-label='What to do with the file' className='export-transcript-mode-grid' role='radiogroup'>
          {modeCards.map((card) => {
            const CardIcon = card.icon;
            const selected = effectiveMode === card.mode;
            return (
              <button
                aria-checked={selected}
                className='export-transcript-mode-card'
                data-selected={selected}
                disabled={busy || isDone}
                key={card.mode}
                onClick={() => chooseMode(card.mode)}
                role='radio'
                type='button'
              >
                <span aria-hidden='true' className='export-transcript-mode-icon'>
                  <CardIcon size={16} stroke={1.75} />
                </span>
                <span className='export-transcript-mode-text'>
                  <strong>{card.title}</strong>
                  <span>{card.description}</span>
                </span>
                <IconCheck aria-hidden='true' className='export-transcript-mode-check' size={14} stroke={2.2} />
              </button>
            );
          })}
        </div>
        {effectiveMode === 'handoff' ? (
          <div className='export-transcript-agent-row'>
            <label className='export-transcript-agent-label' htmlFor={agentSelectId}>
              Continue with
            </label>
            <Select disabled={busy || isDone} onValueChange={setSelectedAgentId} value={effectiveAgentId}>
              <SelectTrigger aria-label='Handoff agent' id={agentSelectId}>
                <SelectValue placeholder='Select agent' />
              </SelectTrigger>
              <SelectContent alignItemWithTrigger={false} className={APP_MODAL_SELECT_CONTENT_CLASS}>
                <SelectGroup>
                  {promptAgents.map((agent) => (
                    <SelectItem key={agent.agentId} value={agent.agentId}>
                      {agent.name}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
        ) : (
          <p className='export-transcript-agent-row export-transcript-agent-hint'>
            The file is saved in the Ghostex exports folder.
          </p>
        )}
        {showResult ? (
          <div className='export-transcript-result'>
            <div className='export-transcript-result-heading'>
              <IconCircleCheckFilled aria-hidden='true' size={16} />
              Saved as Markdown
            </div>
            <div className='export-transcript-path-row'>
              <code className='export-transcript-path'>{stage.stage === 'done' ? stage.path : ''}</code>
              {stage.stage === 'done' && stage.canReveal && onRevealInFinder ? (
                <AppTooltip content='Reveal in Finder'>
                  <Button
                    aria-label='Reveal in Finder'
                    className='export-transcript-reveal-button'
                    onClick={onRevealInFinder}
                    size='icon'
                    type='button'
                    variant='ghost'
                  >
                    <IconFolderSearch aria-hidden='true' size={15} stroke={1.9} />
                  </Button>
                </AppTooltip>
              ) : null}
            </div>
            {actionErrorMessage ? (
              <p className='export-transcript-error' role='alert'>
                {actionErrorMessage}
              </p>
            ) : null}
          </div>
        ) : (
          <>
            <div className='export-transcript-section-title'>Include</div>
            <div className='export-transcript-toggle-list'>
              {INCLUDE_TOGGLE_ROWS.map((row) => (
                <label className='export-transcript-toggle-row' key={row.key}>
                  <span className='export-transcript-toggle-copy'>
                    <span className='export-transcript-toggle-label'>{row.label}</span>
                    <span className='export-transcript-toggle-description'>{row.description}</span>
                  </span>
                  <Switch
                    checked={includeOptions[row.key]}
                    disabled={busy || isDone}
                    onCheckedChange={(checked) => {
                      const next = { ...includeOptions, [row.key]: checked === true };
                      setIncludeOptions(next);
                      writeExportTranscriptIncludeOptions(next);
                    }}
                  />
                </label>
              ))}
            </div>
            {stage.stage === 'failed' ? (
              <p className='export-transcript-error' role='alert'>
                {stage.message}
              </p>
            ) : null}
            {actionErrorMessage ? (
              <p className='export-transcript-error' role='alert'>
                {actionErrorMessage}
              </p>
            ) : null}
          </>
        )}
      </div>
      <AppModalFooter>
        <AppModalButton onClick={onClose} type='button'>
          {showResult ? 'Done' : 'Cancel'}
        </AppModalButton>
        {showResult ? (
          <AppModalButton id={EXPORT_TRANSCRIPT_PRIMARY_ACTION_ID} onClick={copyPath} tone='primary' type='button'>
            {copied ? (
              <IconCheck aria-hidden='true' size={15} stroke={1.9} />
            ) : (
              <IconCopy aria-hidden='true' size={15} stroke={1.9} />
            )}
            {copied ? 'Path Copied' : 'Copy Path'}
          </AppModalButton>
        ) : (
          <AppModalButton
            disabled={!canRun}
            id={EXPORT_TRANSCRIPT_PRIMARY_ACTION_ID}
            onClick={run}
            tone='primary'
            type='button'
          >
            {busy ? (
              <IconLoader2 aria-hidden='true' className='export-transcript-spinner' size={15} stroke={1.9} />
            ) : null}
            {primaryLabel}
          </AppModalButton>
        )}
      </AppModalFooter>
    </AppModalShell>
  );
}
