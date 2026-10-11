/** The chat's composer-side state, as `ghostex-gpui-mobile-chat`'s `ComposerSummary` serializes it. */
export type ComposerSummary = {
  session: string;
  status: string;
  working: boolean;
  composerReady: boolean;
  pendingSend: boolean;
  sendBlockedReason: string | null;
  questionCardVisible: boolean;
  promptKind: string | null;
  approvalAsk: string;
  queueCount: number;
  canQueue: boolean;
  composerCollapsed: boolean;
  placeholder: string;
  draft: string;
  operationError: string | null;
  error: string | null;
  hasRows: boolean;
};

export type SendMode = 'send' | 'queue' | 'compact';

export type SendOutcome =
  | 'sent'
  | 'commandCompleted'
  | 'noSession'
  | 'notReady'
  | 'empty'
  | 'pending'
  | 'blocked'
  | 'cannotQueue';
