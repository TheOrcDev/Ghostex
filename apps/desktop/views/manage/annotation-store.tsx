import { type CSSProperties, type ReactNode } from 'react';
import { playCopySound } from '@/packages/core-ui/copy-sound';
import { IconCircleCheck, IconHelpCircle, IconTestPipe } from '@tabler/icons-react';
import {
  MANAGE_ANNOTATION_IMAGE_MAX_BYTES,
  MANAGE_ANNOTATION_MAX_IMAGES,
  MANAGE_ANNOTATION_SCHEMA_VERSION,
  MANAGE_COMMENT_ANNOTATION_COLOR,
  MANAGE_QUICK_LABELS,
  MANAGE_REDLINE_ANNOTATION_COLOR,
  MANAGE_SELECTION_MAX_LENGTH,
  MANAGE_SELECTION_TOOLBAR_EDGE_MARGIN,
  MANAGE_SELECTION_TOOLBAR_WIDTH_ESTIMATE,
} from './constants';
import {
  ManageAnnotation,
  ManageAnnotationImage,
  ManageAnnotationStore,
  ManageMeoSelectionState,
  ManageQuickLabelId,
  ManageSelectionAnchor,
  isRecord,
} from './types';

export function annotationPersistenceLabel(state: 'idle' | 'loading' | 'ready' | 'saving' | 'saved' | 'error'): string {
  switch (state) {
    case 'error':
      return 'Not saved';
    case 'loading':
      return 'Loading';
    case 'saved':
      return 'Saved';
    case 'saving':
      return 'Saving';
    case 'idle':
    case 'ready':
      return 'Local';
  }
}

export function annotationTypeLabel(annotation: ManageAnnotation): string {
  if (annotation.type === 'redline') {
    return 'Redline';
  }
  if (annotation.labelId) {
    return quickLabelText(annotation.labelId);
  }
  return annotation.scope === 'global' ? 'Global comment' : 'Comment';
}

export function annotationDisplayNote(annotation: ManageAnnotation): string {
  const note = annotation.note.trim();
  if (!note) {
    return '';
  }
  return annotation.labelId && note === quickLabelText(annotation.labelId) ? '' : note;
}

export function quickLabelText(labelId: ManageQuickLabelId): string {
  return MANAGE_QUICK_LABELS.find((label) => label.id === labelId)?.text ?? labelId;
}

export function quickLabelColor(labelId: ManageQuickLabelId | undefined): string {
  return MANAGE_QUICK_LABELS.find((label) => label.id === labelId)?.color ?? MANAGE_COMMENT_ANNOTATION_COLOR;
}

export function manageAnnotationColor(annotation: Pick<ManageAnnotation, 'labelId' | 'type'>): string {
  return annotation.type === 'redline' ? MANAGE_REDLINE_ANNOTATION_COLOR : quickLabelColor(annotation.labelId);
}

export function manageToolbarActionStyle(darkColor: string, lightColor: string): CSSProperties {
  return { '--manage-toolbar-action-color': `light-dark(${lightColor}, ${darkColor})` } as CSSProperties;
}

export function clampManageSelectionToolbarLeft(left: number): number {
  const halfToolbarWidth = Math.min(
    MANAGE_SELECTION_TOOLBAR_WIDTH_ESTIMATE / 2,
    Math.max(0, window.innerWidth / 2 - MANAGE_SELECTION_TOOLBAR_EDGE_MARGIN)
  );
  const minLeft = MANAGE_SELECTION_TOOLBAR_EDGE_MARGIN + halfToolbarWidth;
  const maxLeft = Math.max(minLeft, window.innerWidth - MANAGE_SELECTION_TOOLBAR_EDGE_MARGIN - halfToolbarWidth);
  return Math.min(Math.max(left, minLeft), maxLeft);
}

export function meoSelectionToolbarPosition(
  selectionState: ManageMeoSelectionState,
  fallbackAnchor: ManageSelectionAnchor
): { isBelow: boolean; left: number; top: number } {
  const margin = 8;
  const estimatedWidth = 236;
  const estimatedHeight = 34;
  const anchorX =
    typeof selectionState.anchorX === 'number' && Number.isFinite(selectionState.anchorX)
      ? selectionState.anchorX
      : fallbackAnchor.left;
  const anchorY =
    typeof selectionState.anchorY === 'number' && Number.isFinite(selectionState.anchorY)
      ? selectionState.anchorY
      : fallbackAnchor.top;
  const anchorBottomY =
    typeof selectionState.anchorBottomY === 'number' && Number.isFinite(selectionState.anchorBottomY)
      ? selectionState.anchorBottomY
      : fallbackAnchor.top;
  const rawLeft = selectionState.align === 'start' ? anchorX : anchorX - estimatedWidth / 2;
  const maxLeft = Math.max(margin, window.innerWidth - estimatedWidth - margin);
  /* The formatting bar floats at the bottom, so the document header is the top chrome a selection toolbar must clear. */
  const toolbarBottom =
    (document.querySelector('.manage-preview-header') as HTMLElement | null)?.getBoundingClientRect().bottom ?? 0;
  const aboveTop = anchorY - margin - estimatedHeight;
  const isBelow = aboveTop < toolbarBottom + margin;
  return {
    isBelow,
    left: Math.min(maxLeft, Math.max(margin, rawLeft)),
    top: Math.max(margin, isBelow ? anchorBottomY + margin : anchorY - margin),
  };
}

export function renderManageQuickLabelIcon(labelId: ManageQuickLabelId): ReactNode {
  switch (labelId) {
    case 'clarify':
      return <IconHelpCircle aria-hidden='true' size={15} />;
    case 'needs-tests':
      return <IconTestPipe aria-hidden='true' size={15} />;
    case 'looks-good':
      return <IconCircleCheck aria-hidden='true' size={15} />;
  }
}

export function normalizeAnnotationQuote(text: string): string {
  return text.replace(/\s+/g, ' ').trim().slice(0, MANAGE_SELECTION_MAX_LENGTH);
}

export function selectionAnchorFromRect(rect: DOMRect | undefined): ManageSelectionAnchor | undefined {
  if (!rect || rect.width === 0 || rect.height === 0) {
    return undefined;
  }
  const left = Math.min(Math.max(rect.left + rect.width / 2, 12), window.innerWidth - 12);
  const top = Math.min(Math.max(rect.top, 12), window.innerHeight - 12);
  return { left, top };
}

export function defaultManageSelectionAnchor(): ManageSelectionAnchor {
  return {
    left: Math.min(Math.max(window.innerWidth / 2, 12), window.innerWidth - 12),
    top: Math.min(Math.max(72, 12), window.innerHeight - 12),
  };
}

export function annotationPreviewText(annotation: ManageAnnotation): string {
  const note = annotationDisplayNote(annotation);
  if (note) {
    return truncateManageAnnotationPreviewText(note);
  }
  if (annotation.labelId) {
    return quickLabelText(annotation.labelId);
  }
  if (annotation.type === 'redline') {
    return 'Marked for deletion';
  }
  return truncateManageAnnotationPreviewText(annotation.quote);
}

export function truncateManageAnnotationPreviewText(text: string): string {
  const normalized = normalizeAnnotationQuote(text);
  return normalized.length > 150 ? `${normalized.slice(0, 147)}...` : normalized;
}

export function annotationPreviewCardStyle(anchor: ManageSelectionAnchor): CSSProperties {
  const width = Math.min(320, Math.max(240, window.innerWidth - 24));
  const halfWidth = width / 2;
  return {
    left: Math.min(Math.max(anchor.left, 12 + halfWidth), window.innerWidth - 12 - halfWidth),
    top: Math.max(12, anchor.top - 96),
    width,
  };
}

export function commentPopoverStyle(anchor: ManageSelectionAnchor): CSSProperties {
  const width = Math.min(360, Math.max(280, window.innerWidth - 24));
  const left = Math.min(Math.max(anchor.left - width / 2, 12), window.innerWidth - width - 12);
  const maxTop = Math.max(12, window.innerHeight - 260);
  const top = Math.min(Math.max(anchor.top + 12, 12), maxTop);
  return {
    left,
    top,
    width,
  };
}

export function normalizeAttachmentName(name: string): string {
  const trimmed = name.trim().replace(/\s+/g, '-');
  return trimmed ? trimmed.slice(0, 80) : 'image';
}

export function parseManageAnnotationStore(content: string): Record<string, ManageAnnotation[]> {
  if (!content.trim()) {
    return {};
  }
  try {
    const value = JSON.parse(content) as unknown;
    if (!isRecord(value)) {
      return {};
    }
    const annotationsValue = value.annotationsByPath;
    if (!isRecord(annotationsValue)) {
      return {};
    }
    const normalized: Record<string, ManageAnnotation[]> = {};
    for (const [path, annotations] of Object.entries(annotationsValue)) {
      const normalizedPath = normalizeStoredAnnotationPath(path);
      if (!normalizedPath || !Array.isArray(annotations)) {
        continue;
      }
      const normalizedAnnotations = annotations
        .map((annotation) => normalizeStoredAnnotation(annotation))
        .filter((annotation): annotation is ManageAnnotation => Boolean(annotation));
      if (normalizedAnnotations.length > 0) {
        normalized[normalizedPath] = normalizedAnnotations;
      }
    }
    return normalized;
  } catch {
    return {};
  }
}

export function serializeManageAnnotationStore(annotationsByPath: Record<string, ManageAnnotation[]>): string {
  const store: ManageAnnotationStore = {
    annotationsByPath,
    updatedAt: new Date().toISOString(),
    version: MANAGE_ANNOTATION_SCHEMA_VERSION,
  };
  return `${JSON.stringify(store, null, 2)}\n`;
}

export function stableManageAnnotationStoreKey(annotationsByPath: Record<string, ManageAnnotation[]>): string {
  return JSON.stringify(annotationsByPath);
}

export function normalizeStoredAnnotationPath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed || trimmed.startsWith('/') || trimmed.includes('\0')) {
    return '';
  }
  const components = trimmed.split('/').filter(Boolean);
  if (components.includes('.') || components.includes('..')) {
    return '';
  }
  return components.join('/');
}

export function normalizeStoredAnnotation(value: unknown): ManageAnnotation | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  const type = value.type === 'redline' ? 'redline' : value.type === 'comment' ? 'comment' : undefined;
  if (!type) {
    return undefined;
  }
  const quote = typeof value.quote === 'string' ? normalizeAnnotationQuote(value.quote) : '';
  const note = typeof value.note === 'string' ? value.note.slice(0, 4_000) : '';
  const attachments = Array.isArray(value.attachments)
    ? value.attachments
        .map((attachment) => normalizeStoredAttachment(attachment))
        .filter((attachment): attachment is ManageAnnotationImage => Boolean(attachment))
        .slice(0, MANAGE_ANNOTATION_MAX_IMAGES)
    : [];
  if (type === 'redline' && !quote) {
    return undefined;
  }
  if (type === 'comment' && !quote && !note.trim() && attachments.length === 0) {
    return undefined;
  }
  const labelId = normalizeQuickLabelId(value.labelId);
  const updatedAt = normalizeStoredTimestamp(value.updatedAt);
  const sentAt = normalizeStoredTimestamp(value.sentAt);
  return {
    attachments,
    createdAt: typeof value.createdAt === 'string' ? value.createdAt : new Date().toISOString(),
    id: typeof value.id === 'string' && value.id.trim() ? value.id : `manage-annotation-${Date.now()}`,
    ...(labelId ? { labelId } : {}),
    note,
    quote,
    scope: quote ? 'selection' : 'global',
    type,
    ...(updatedAt ? { updatedAt } : {}),
    ...(sentAt ? { sentAt } : {}),
  };
}

export function normalizeStoredTimestamp(value: unknown): string | undefined {
  return typeof value === 'string' && Number.isFinite(Date.parse(value)) ? value : undefined;
}

/** Never sent, or edited after its last send. Coverage follows each note's own last send, not the last batch. */
export function isManageAnnotationPending(annotation: ManageAnnotation): boolean {
  if (!annotation.sentAt) {
    return true;
  }
  const edited = Date.parse(annotation.updatedAt ?? annotation.createdAt);
  const sent = Date.parse(annotation.sentAt);
  if (!Number.isFinite(sent)) {
    return true;
  }
  return Number.isFinite(edited) && edited > sent;
}

export function pendingManageAnnotations(annotations: readonly ManageAnnotation[]): ManageAnnotation[] {
  return annotations.filter(isManageAnnotationPending);
}

export function sentManageAnnotations(annotations: readonly ManageAnnotation[]): ManageAnnotation[] {
  return annotations.filter((annotation) => !isManageAnnotationPending(annotation));
}

export type ManageAnnotationReviewCounts = { pending: number; sent: number };

export function manageAnnotationReviewCounts(annotations: readonly ManageAnnotation[]): ManageAnnotationReviewCounts {
  let pending = 0;
  let sent = 0;
  for (const annotation of annotations) {
    if (isManageAnnotationPending(annotation)) {
      pending += 1;
    } else {
      sent += 1;
    }
  }
  return { pending, sent };
}

/**
 * A timestamp strictly later than `after`, so an edit made in the same
 * millisecond as a send (or with a clock that moved backwards) still counts as
 * pending.
 */
export function manageAnnotationTimestampAfter(after: string | undefined): string {
  const now = Date.now();
  const floor = after ? Date.parse(after) : Number.NaN;
  return new Date(Number.isFinite(floor) && floor >= now ? floor + 1 : now).toISOString();
}

export function normalizeStoredAttachment(value: unknown): ManageAnnotationImage | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  const dataUrl = typeof value.dataUrl === 'string' ? value.dataUrl : '';
  const mimeType = typeof value.mimeType === 'string' ? value.mimeType : '';
  const name = typeof value.name === 'string' ? normalizeAttachmentName(value.name) : 'image';
  const size = typeof value.size === 'number' && Number.isFinite(value.size) ? Math.max(0, value.size) : 0;
  if (
    !dataUrl.startsWith('data:image/') ||
    !mimeType.startsWith('image/') ||
    size > MANAGE_ANNOTATION_IMAGE_MAX_BYTES
  ) {
    return undefined;
  }
  return {
    dataUrl,
    id: typeof value.id === 'string' && value.id.trim() ? value.id : `manage-annotation-image-${Date.now()}`,
    mimeType,
    name,
    size,
  };
}

export function normalizeQuickLabelId(value: unknown): ManageQuickLabelId | undefined {
  return MANAGE_QUICK_LABELS.some((label) => label.id === value) ? (value as ManageQuickLabelId) : undefined;
}

export function findManageAnnotationTextMatches(text: string, quote: string): Array<{ from: number; to: number }> {
  const normalizedQuote = normalizeAnnotationQuote(quote);
  if (!normalizedQuote) {
    return [];
  }
  const normalizedText = buildManageNormalizedTextIndex(text);
  const matches: Array<{ from: number; to: number }> = [];
  let fromIndex = 0;
  while (fromIndex < normalizedText.text.length) {
    const matchIndex = normalizedText.text.indexOf(normalizedQuote, fromIndex);
    if (matchIndex < 0) {
      break;
    }
    const start = normalizedText.positions[matchIndex];
    const end = normalizedText.positions[matchIndex + normalizedQuote.length - 1];
    if (typeof start === 'number' && typeof end === 'number' && end >= start) {
      matches.push({ from: start, to: end + 1 });
    }
    fromIndex = matchIndex + normalizedQuote.length;
  }
  return matches;
}

export function buildManageNormalizedTextIndex(text: string): { positions: number[]; text: string } {
  const positions: number[] = [];
  let normalized = '';
  let previousWasWhitespace = true;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index] ?? '';
    if (/\s/u.test(character)) {
      if (!previousWasWhitespace) {
        normalized += ' ';
        positions.push(index);
        previousWasWhitespace = true;
      }
      continue;
    }
    normalized += character;
    positions.push(index);
    previousWasWhitespace = false;
  }
  if (normalized.endsWith(' ')) {
    normalized = normalized.slice(0, -1);
    positions.pop();
  }
  return { positions, text: normalized };
}

export async function writeTextToClipboard(text: string): Promise<void> {
  playCopySound();
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.cssText = 'position:fixed;left:-9999px;top:-9999px';
    document.body.append(textarea);
    textarea.select();
    const didCopy = document.execCommand('copy');
    textarea.remove();
    if (!didCopy) {
      throw new Error('Clipboard copy failed.');
    }
  }
}
