import {
  IconAlertTriangle,
  IconCheck,
  IconChecklist,
  IconCopy,
  IconEdit,
  IconFileText,
  IconMessagePlus,
  IconMessages,
  IconRefresh,
  IconSend,
  IconTrash,
  IconX,
} from '@tabler/icons-react';
import { formatSidebarHotkeyLabel } from '@/packages/core-ui/hotkey-label';
import { type ProjectDocsFilePreview as ManageFilePreview } from '@/packages/shared/project-docs';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { MANAGE_ANNOTATION_IMAGE_MAX_BYTES, MANAGE_ANNOTATION_MAX_IMAGES, MANAGE_QUICK_LABELS } from '../constants';
import {
  ManageAnnotation,
  ManageAnnotationImage,
  ManageAnnotationPreview,
  ManageAnnotationSendState,
  ManageAnnotationSendTarget,
  ManageAnnotationType,
  ManageCapturedSelection,
  ManageCommentDraft,
  ManageQuickLabel,
  ManageQuickLabelId,
  ManageReviewDocument,
  ManageSelectionAnchor,
  ManageSelectionToolbarMode,
} from '../types';
import {
  ManageAnnotationDropdown,
  ManageAnnotationPreviewCard,
  ManageAnnotationToolbar,
  ManageCommentPopover,
  ManageReviewMenu,
} from './annotation-overlays';
import {
  DeferredDrawingEditor as ManageExcalidrawEditor,
  DeferredMarkdownEditor as ManageMarkdownReviewViewer,
} from './deferred-editors';
import { ManageHtmlRenderViewer } from './html-viewer';
import { ManagePreviewMessage, isEditableEventTarget } from './preview-shared';
import { ManageTextEditor } from './text-editor';
import { ManageTooltipButton } from '../manage-tooltip-button';
import { ManageDocumentTitle } from './document-title';
import {
  type ManageAnnotationReviewCounts,
  defaultManageSelectionAnchor,
  manageAnnotationReviewCounts,
  normalizeAnnotationQuote,
  normalizeAttachmentName,
  selectionAnchorFromRect,
  writeTextToClipboard,
} from '../annotation-store';
import { formatManageAnnotationFeedback } from '../annotation-feedback';
import { formatFileSize, isExcalidrawPath, isHtmlPath, isMarkdownPath, languageLabelForPath } from '../file-tree-utils';

export function ManagePreview({
  annotations,
  draftContent,
  error,
  folderPendingFeedback,
  hasExternalChanges,
  isDirty,
  onAnnotationsChange,
  onCloseReviewDocument,
  onDraftContentChange,
  onEditAnnotationNote,
  onOpenDocument,
  onReload,
  onSendFeedback,
  preview,
  previewState,
  reviewDocument,
  saveState,
  selectedPath,
  sendState,
  sendTarget,
}: {
  annotations: ManageAnnotation[];
  draftContent: string;
  error?: string;
  folderPendingFeedback: { count: number; fileCount: number };
  hasExternalChanges: boolean;
  isDirty: boolean;
  onAnnotationsChange: (updater: (annotations: ManageAnnotation[]) => ManageAnnotation[]) => void;
  onCloseReviewDocument: () => void;
  onDraftContentChange: (content: string) => void;
  onEditAnnotationNote: (annotationId: string, note: string) => void;
  onOpenDocument: (path: string) => void;
  onReload: () => void;
  onSendFeedback: (request: { allFiles?: boolean; scope: 'all' | 'pending' }) => Promise<void>;
  preview?: ManageFilePreview;
  previewState: 'idle' | 'loading' | 'ready' | 'error';
  reviewDocument?: ManageReviewDocument;
  saveState: 'idle' | 'saving' | 'saved' | 'error';
  selectedPath?: string;
  sendState: ManageAnnotationSendState;
  sendTarget: ManageAnnotationSendTarget | null;
}) {
  const reviewCounts = useMemo(() => manageAnnotationReviewCounts(annotations), [annotations]);
  const [selection, setSelection] = useState<ManageCapturedSelection>();
  const [selectionToolbarMode, setSelectionToolbarMode] = useState<ManageSelectionToolbarMode>('annotations');
  const [commentDraft, setCommentDraft] = useState<ManageCommentDraft>();
  const [annotationPreview, setAnnotationPreview] = useState<ManageAnnotationPreview>();
  const [feedbackCopyState, setFeedbackCopyState] = useState<'idle' | 'copied' | 'error'>('idle');
  const [clearAnnotationsConfirming, setClearAnnotationsConfirming] = useState(false);
  const [annotationsDropdownOpen, setAnnotationsDropdownOpen] = useState(false);
  const [reviewMenuOpen, setReviewMenuOpen] = useState(false);
  const [editingAnnotationId, setEditingAnnotationId] = useState<string>();
  const [htmlAnnotationEnabled, setHtmlAnnotationEnabled] = useState(true);
  const annotationsDropdownRef = useRef<HTMLDivElement | null>(null);
  const reviewMenuRef = useRef<HTMLDivElement | null>(null);
  const clearAnnotationsTimerRef = useRef<number | undefined>(undefined);
  const selectedPathRef = useRef<string | undefined>(selectedPath);

  const resetClearAnnotationsConfirm = useCallback(() => {
    if (clearAnnotationsTimerRef.current !== undefined) {
      window.clearTimeout(clearAnnotationsTimerRef.current);
      clearAnnotationsTimerRef.current = undefined;
    }
    setClearAnnotationsConfirming(false);
  }, []);

  useEffect(() => {
    if (selectedPathRef.current !== selectedPath) {
      selectedPathRef.current = selectedPath;
      setSelection(undefined);
      setSelectionToolbarMode('annotations');
      setCommentDraft(undefined);
      setAnnotationPreview(undefined);
      setFeedbackCopyState('idle');
      resetClearAnnotationsConfirm();
      setAnnotationsDropdownOpen(false);
      setReviewMenuOpen(false);
      setEditingAnnotationId(undefined);
    }
  }, [resetClearAnnotationsConfirm, selectedPath]);

  useEffect(() => {
    if (annotations.length === 0) {
      resetClearAnnotationsConfirm();
    }
  }, [annotations.length, resetClearAnnotationsConfirm]);

  useEffect(() => {
    if (!reviewMenuOpen) {
      return;
    }
    function handlePointerDown(event: PointerEvent) {
      const menuElement = reviewMenuRef.current;
      if (!menuElement || !event.target || menuElement.contains(event.target as Node)) {
        return;
      }
      setReviewMenuOpen(false);
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setReviewMenuOpen(false);
      }
    }
    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [reviewMenuOpen]);

  /*
   * CDXC:Docs 2026-09-15 DECISION:
   * User: the Send button can be pressed again and again. It sends the new notes while there are any, and once everything has been sent it sends all the notes again, instead of going dark with "Nothing new to send".
   */
  const sendFeedback = useCallback(() => {
    setReviewMenuOpen(false);
    void onSendFeedback({ scope: reviewCounts.pending > 0 ? 'pending' : 'all' });
  }, [onSendFeedback, reviewCounts.pending]);

  /*
   * Cmd/Ctrl+Enter presses Send from anywhere in the document, the same chord
   * that adds a note inside the composer. The composer stops the chord from
   * bubbling, so the two never fire for one keypress.
   */
  useEffect(() => {
    if (commentDraft || annotations.length === 0) {
      return;
    }
    function handleSendShortcut(event: KeyboardEvent) {
      if (event.isComposing || !(event.metaKey || event.ctrlKey) || event.altKey || event.key !== 'Enter') {
        return;
      }
      event.preventDefault();
      sendFeedback();
    }
    window.addEventListener('keydown', handleSendShortcut);
    return () => window.removeEventListener('keydown', handleSendShortcut);
  }, [annotations.length, commentDraft, sendFeedback]);

  useEffect(
    () => () => {
      if (clearAnnotationsTimerRef.current !== undefined) {
        window.clearTimeout(clearAnnotationsTimerRef.current);
      }
    },
    []
  );

  useEffect(() => {
    if (!annotationsDropdownOpen) {
      return;
    }
    function handlePointerDown(event: PointerEvent) {
      const dropdownElement = annotationsDropdownRef.current;
      if (!dropdownElement || !event.target || dropdownElement.contains(event.target as Node)) {
        return;
      }
      setAnnotationsDropdownOpen(false);
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setAnnotationsDropdownOpen(false);
      }
    }
    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [annotationsDropdownOpen]);

  const addAnnotation = useCallback(
    ({
      attachments = [],
      labelId,
      note = '',
      quote = '',
      type,
    }: {
      attachments?: ManageAnnotationImage[];
      labelId?: ManageQuickLabelId;
      note?: string;
      quote?: string;
      type: ManageAnnotationType;
    }) => {
      const normalizedQuote = normalizeAnnotationQuote(quote);
      if (type === 'redline' && !normalizedQuote) {
        return;
      }
      const normalizedNote = note.trim();
      if (type === 'comment' && !normalizedQuote && !normalizedNote && attachments.length === 0) {
        return;
      }
      const nextAnnotation: ManageAnnotation = {
        attachments,
        createdAt: new Date().toISOString(),
        id: `manage-annotation-${Date.now()}-${Math.random().toString(16).slice(2)}`,
        labelId,
        note: normalizedNote,
        quote: normalizedQuote,
        scope: normalizedQuote ? 'selection' : 'global',
        type,
      };
      onAnnotationsChange((current) => [...current, nextAnnotation]);
      setSelection(undefined);
      setSelectionToolbarMode('annotations');
      setCommentDraft(undefined);
    },
    [onAnnotationsChange]
  );

  const captureSelectedText = useCallback((capturedSelection: ManageCapturedSelection) => {
    const normalized = normalizeAnnotationQuote(capturedSelection.text);
    if (!normalized) {
      return;
    }
    setAnnotationPreview(undefined);
    setCommentDraft(undefined);
    setSelectionToolbarMode('annotations');
    setSelection({
      anchor: capturedSelection.anchor,
      text: normalized,
    });
  }, []);

  const clearSelectedText = useCallback(() => {
    setSelection(undefined);
    setSelectionToolbarMode('annotations');
  }, []);

  const openCommentDraft = useCallback((quote: string, anchor: ManageSelectionAnchor, initialNote = '') => {
    setAnnotationPreview(undefined);
    setSelection(undefined);
    setSelectionToolbarMode('annotations');
    setEditingAnnotationId(undefined);
    setCommentDraft({
      anchor,
      attachmentError: '',
      attachments: [],
      note: initialNote,
      quote: normalizeAnnotationQuote(quote),
    });
  }, []);

  const addSelectedRedline = useCallback(() => {
    if (!selection) {
      return;
    }
    addAnnotation({
      quote: selection.text,
      type: 'redline',
    });
  }, [addAnnotation, selection]);

  const addQuickLabel = useCallback(
    (label: ManageQuickLabel) => {
      addAnnotation({
        labelId: label.id,
        note: '',
        quote: selection?.text ?? commentDraft?.quote ?? '',
        type: 'comment',
      });
    },
    [addAnnotation, commentDraft?.quote, selection?.text]
  );

  const submitCommentDraft = useCallback(() => {
    if (!commentDraft) {
      return;
    }
    if (editingAnnotationId) {
      onEditAnnotationNote(editingAnnotationId, commentDraft.note);
      setEditingAnnotationId(undefined);
      setCommentDraft(undefined);
      return;
    }
    addAnnotation({
      attachments: commentDraft.attachments,
      note: commentDraft.note,
      quote: commentDraft.quote,
      type: 'comment',
    });
  }, [addAnnotation, commentDraft, editingAnnotationId, onEditAnnotationNote]);

  const editAnnotation = useCallback(
    (annotationId: string) => {
      const annotation = annotations.find((candidate) => candidate.id === annotationId);
      if (!annotation) {
        return;
      }
      setAnnotationsDropdownOpen(false);
      setAnnotationPreview(undefined);
      setSelection(undefined);
      setSelectionToolbarMode('annotations');
      setEditingAnnotationId(annotationId);
      setCommentDraft({
        anchor: defaultManageSelectionAnchor(),
        attachmentError: '',
        attachments: annotation.attachments,
        note: annotation.note,
        quote: annotation.quote,
      });
    },
    [annotations]
  );

  const updateCommentDraftNote = useCallback((note: string) => {
    setCommentDraft((current) => (current ? { ...current, note } : current));
  }, []);

  const addAttachmentFiles = useCallback((files: FileList | File[]) => {
    const imageFiles = Array.from(files).filter((file) => file.type.startsWith('image/'));
    if (imageFiles.length === 0) {
      return;
    }
    setCommentDraft((current) => {
      if (!current) {
        return current;
      }
      const availableSlots = Math.max(0, MANAGE_ANNOTATION_MAX_IMAGES - current.attachments.length);
      if (availableSlots === 0) {
        return {
          ...current,
          attachmentError: `Use ${MANAGE_ANNOTATION_MAX_IMAGES} images or fewer per annotation.`,
        };
      }
      let attachmentError =
        imageFiles.length > availableSlots ? `Use ${MANAGE_ANNOTATION_MAX_IMAGES} images or fewer per annotation.` : '';
      for (const file of imageFiles.slice(0, availableSlots)) {
        if (file.size > MANAGE_ANNOTATION_IMAGE_MAX_BYTES) {
          attachmentError = 'Images must be 512 KB or smaller.';
          continue;
        }
        const reader = new FileReader();
        reader.onload = () => {
          const dataUrl = typeof reader.result === 'string' ? reader.result : '';
          if (!dataUrl) {
            return;
          }
          setCommentDraft((latest) => {
            if (!latest || latest.attachments.length >= MANAGE_ANNOTATION_MAX_IMAGES) {
              return latest;
            }
            return {
              ...latest,
              attachmentError: '',
              attachments: [
                ...latest.attachments,
                {
                  dataUrl,
                  id: `manage-annotation-image-${Date.now()}-${Math.random().toString(16).slice(2)}`,
                  mimeType: file.type,
                  name: normalizeAttachmentName(file.name),
                  size: file.size,
                },
              ],
            };
          });
        };
        reader.onerror = () => {
          setCommentDraft((latest) =>
            latest ? { ...latest, attachmentError: 'Could not read image attachment.' } : latest
          );
        };
        reader.readAsDataURL(file);
      }
      return {
        ...current,
        attachmentError,
      };
    });
  }, []);

  const removeDraftAttachment = useCallback((attachmentId: string) => {
    setCommentDraft((current) =>
      current
        ? {
            ...current,
            attachments: current.attachments.filter((attachment) => attachment.id !== attachmentId),
          }
        : current
    );
  }, []);

  const copyFeedback = useCallback(async () => {
    if (!selectedPath) {
      return;
    }
    /*
     * CDXC:Docs 2026-08-10:
     * This markdown is read by a human and by the agent it is pasted to, so it
     * names the file the way the tree does rather than by routing address.
     */
    const output = formatManageAnnotationFeedback(
      [{ annotations, content: draftContent, name: preview?.displayPath ?? selectedPath }],
      'all'
    ).text;
    try {
      await writeTextToClipboard(output);
      setFeedbackCopyState('copied');
      window.setTimeout(() => setFeedbackCopyState('idle'), 1_600);
    } catch {
      setFeedbackCopyState('error');
    }
  }, [annotations, draftContent, preview?.displayPath, selectedPath]);

  const clearAllAnnotations = useCallback(() => {
    if (annotations.length === 0) {
      resetClearAnnotationsConfirm();
      return;
    }
    if (!clearAnnotationsConfirming) {
      setClearAnnotationsConfirming(true);
      if (clearAnnotationsTimerRef.current !== undefined) {
        window.clearTimeout(clearAnnotationsTimerRef.current);
      }
      clearAnnotationsTimerRef.current = window.setTimeout(() => {
        clearAnnotationsTimerRef.current = undefined;
        setClearAnnotationsConfirming(false);
      }, 3_000);
      return;
    }
    resetClearAnnotationsConfirm();
    setAnnotationsDropdownOpen(false);
    onAnnotationsChange(() => []);
  }, [annotations.length, clearAnnotationsConfirming, onAnnotationsChange, resetClearAnnotationsConfirm]);

  const openCommentForSelection = useCallback(() => {
    if (!selection) {
      return;
    }
    openCommentDraft(selection.text, selection.anchor);
  }, [openCommentDraft, selection]);

  const openGlobalComment = useCallback(
    (anchor: ManageSelectionAnchor) => {
      openCommentDraft('', anchor);
    },
    [openCommentDraft]
  );

  useEffect(() => {
    if (!selection || commentDraft) {
      return;
    }
    const activeSelection = selection;
    function handleAnnotationShortcut(event: KeyboardEvent) {
      if (event.isComposing || event.metaKey || event.ctrlKey || event.altKey || isEditableEventTarget(event.target)) {
        return;
      }
      const key = event.key.toLocaleLowerCase();
      if (key === 'escape') {
        event.preventDefault();
        setSelection(undefined);
        return;
      }
      if (key === 'backspace' || key === 'd' || key === 'delete') {
        event.preventDefault();
        addSelectedRedline();
        return;
      }
      if (key === 'c') {
        event.preventDefault();
        openCommentForSelection();
        return;
      }
      if (/^[1-3]$/u.test(key)) {
        event.preventDefault();
        const label = MANAGE_QUICK_LABELS[Number(key) - 1];
        if (label) {
          addQuickLabel(label);
        }
        return;
      }
      if (event.key.length === 1) {
        event.preventDefault();
        openCommentDraft(activeSelection.text, activeSelection.anchor, event.key);
      }
    }
    window.addEventListener('keydown', handleAnnotationShortcut);
    return () => window.removeEventListener('keydown', handleAnnotationShortcut);
  }, [addQuickLabel, addSelectedRedline, commentDraft, openCommentDraft, openCommentForSelection, selection]);

  const removeAnnotation = useCallback(
    (annotationId: string) => {
      onAnnotationsChange((current) => current.filter((annotation) => annotation.id !== annotationId));
    },
    [onAnnotationsChange]
  );

  const removePreviewAnnotation = useCallback(
    (annotationId: string) => {
      removeAnnotation(annotationId);
      setAnnotationPreview(undefined);
    },
    [removeAnnotation]
  );

  if (previewState === 'loading') {
    return <ManagePreviewMessage icon={<IconRefresh aria-hidden='true' size={20} />} title='Loading file' />;
  }
  if (error) {
    return <ManagePreviewMessage icon={<IconAlertTriangle aria-hidden='true' size={21} />} title={error} />;
  }
  if (!selectedPath || !preview) {
    return <ManagePreviewMessage icon={<IconFileText aria-hidden='true' size={21} />} title='Select a file' />;
  }

  const language = languageLabelForPath(preview.path);
  const isMarkdown = isMarkdownPath(preview.path);
  const isReview = Boolean(reviewDocument);
  const sendLabel = manageSendButtonLabel(sendState, sendTarget, reviewCounts);
  const isDrawing = isExcalidrawPath(preview.path);
  const isHtml = isHtmlPath(preview.path);
  const usesCompactArtifactHeader = isMarkdown || isDrawing || isHtml;
  /*
   * CDXC:Docs 2026-08-09:
   * Show the file the way the tree names it. `preview.path` is a routing
   * address that starts with the reserved mount segment for anything under a
   * configured Docs directory, which is not a name any human asked for.
   */
  const previewDisplayPath = preview.displayPath ?? preview.path;
  const previewTitle = usesCompactArtifactHeader ? previewDisplayPath : preview.name;
  return (
    <div
      className='manage-preview-content'
      data-compact-header={String(usesCompactArtifactHeader)}
      data-kind={isMarkdown ? 'markdown' : isDrawing ? 'drawing' : isHtml ? 'html' : 'text'}
    >
      <header className='manage-preview-header'>
        <ManageDocumentTitle
          dirty={isDirty && !isReview}
          key={preview.path}
          title={previewTitle}
          icon={
            isDrawing ? (
              <IconEdit aria-hidden='true' size={17} stroke={1.85} />
            ) : (
              <IconFileText aria-hidden='true' size={17} stroke={1.85} />
            )
          }
        />
        <div className='manage-preview-meta'>
          {isReview ? <span>Agent reply</span> : <span>{language}</span>}
          {isReview && reviewDocument?.sessionTitle ? <span>{reviewDocument.sessionTitle}</span> : null}
          {!isReview && preview.size !== undefined ? <span>{formatFileSize(preview.size)}</span> : null}
          {isDirty ? <span>Edited</span> : saveState === 'saved' ? <span>Saved</span> : null}
        </div>
        {isMarkdown ? (
          <div className='manage-preview-header-actions'>
            {/*
              CDXC:Docs 2026-09-14 DECISION:
              User: keep the actions ordered from right to left as files-list toggle, Reload, Clear, Copy, Add global comment, and Annotations list; use a trash icon for Clear and label the annotations tooltip "Annotations list".
              Match the supplied compact, borderless toolbar reference, superseding the earlier segmented button widths; the hidden sidebar's expand button retains its requested 41px width.
            */}
            <div className='manage-annotation-dropdown-shell' ref={annotationsDropdownRef}>
              <ManageTooltipButton
                aria-controls='manage-markdown-annotation-dropdown'
                aria-expanded={annotationsDropdownOpen}
                aria-haspopup='dialog'
                aria-label='Show annotations'
                className='manage-annotation-dropdown-trigger'
                onClick={() => setAnnotationsDropdownOpen((current) => !current)}
                tooltip='Annotations list'
                type='button'
              >
                <IconMessages aria-hidden='true' size={14} />
                <span className='manage-count-badge'>{annotations.length}</span>
              </ManageTooltipButton>
              {annotationsDropdownOpen ? (
                <ManageAnnotationDropdown
                  annotations={annotations}
                  onEditAnnotation={editAnnotation}
                  onRemoveAnnotation={removeAnnotation}
                />
              ) : null}
            </div>
            <ManageTooltipButton
              aria-label='Add global comment'
              className='manage-add-global-comment-button'
              onClick={(event) =>
                openGlobalComment(
                  selectionAnchorFromRect(event.currentTarget.getBoundingClientRect()) ?? defaultManageSelectionAnchor()
                )
              }
              tooltip='Add global comment'
              type='button'
            >
              <IconMessagePlus aria-hidden='true' size={14} />
              <span>Comment</span>
            </ManageTooltipButton>
            {/*
              CDXC:Docs 2026-09-15 DECISION:
              User: feedback goes straight to the agent. The Send button names where it will land before it is pressed (the agent and session last clicked in the sidebar, and whether that lands in its chat or its terminal), and reads "Copy" when the app would put it on the clipboard instead.
              The Review menu beside it carries Resend all and, with notes in several files, Send new across all files. There is no Finish review, Undo finish, or Archive: Docs is a side pane, not a review session (this supersedes the same-day Herdr Annotate review loop).
            */}
            <ManageTooltipButton
              aria-label={sendLabel.tooltip}
              className='manage-send-feedback-button'
              data-state={sendState.kind}
              disabled={annotations.length === 0 || sendState.kind === 'sending'}
              onClick={sendFeedback}
              tooltip={sendLabel.tooltip}
              type='button'
            >
              {sendState.kind === 'sent' ? (
                <IconCheck aria-hidden='true' size={14} />
              ) : (
                <IconSend aria-hidden='true' size={14} />
              )}
              <span>{sendLabel.text}</span>
            </ManageTooltipButton>
            <div className='manage-review-menu-shell' ref={reviewMenuRef}>
              <ManageTooltipButton
                aria-controls='manage-markdown-review-menu'
                aria-expanded={reviewMenuOpen}
                aria-haspopup='menu'
                aria-label='Review actions'
                className='manage-review-menu-trigger'
                disabled={annotations.length === 0 && folderPendingFeedback.count === 0}
                onClick={() => setReviewMenuOpen((current) => !current)}
                tooltip='Review actions'
                type='button'
              >
                <IconChecklist aria-hidden='true' size={14} />
              </ManageTooltipButton>
              {reviewMenuOpen ? (
                <ManageReviewMenu
                  counts={reviewCounts}
                  folderPending={isReview ? { count: 0, fileCount: 0 } : folderPendingFeedback}
                  onResendAll={() => {
                    setReviewMenuOpen(false);
                    void onSendFeedback({ scope: 'all' });
                  }}
                  onSendAcrossFiles={() => {
                    setReviewMenuOpen(false);
                    void onSendFeedback({ allFiles: true, scope: 'pending' });
                  }}
                />
              ) : null}
            </div>
            <ManageTooltipButton
              aria-label='Copy feedback'
              className='manage-copy-feedback-button'
              disabled={annotations.length === 0}
              onClick={() => void copyFeedback()}
              tooltip='Copy feedback'
              type='button'
            >
              {feedbackCopyState === 'copied' ? (
                <IconCheck aria-hidden='true' size={14} />
              ) : (
                <IconCopy aria-hidden='true' size={14} />
              )}
              <span>{feedbackCopyState === 'copied' ? 'Copied' : 'Copy'}</span>
            </ManageTooltipButton>
            <ManageTooltipButton
              aria-label='Clear all annotations'
              className='manage-clear-annotations-button'
              data-confirming={String(clearAnnotationsConfirming)}
              disabled={annotations.length === 0}
              onClick={clearAllAnnotations}
              tooltip='Clear All Annotations'
              type='button'
            >
              <IconTrash aria-hidden='true' size={14} />
              <span>{clearAnnotationsConfirming ? 'Confirm' : 'Clear'}</span>
            </ManageTooltipButton>
            {isReview ? (
              <ManageTooltipButton
                aria-label='Close reply review'
                className='manage-file-reload-button manage-review-close-button'
                onClick={onCloseReviewDocument}
                tooltip='Close reply review'
                type='button'
              >
                <IconX aria-hidden='true' size={14} />
              </ManageTooltipButton>
            ) : (
              <ManageTooltipButton
                aria-label={hasExternalChanges ? 'Reload file with new changes' : 'Reload file'}
                className='manage-file-reload-button'
                data-changes-available={String(hasExternalChanges)}
                onClick={onReload}
                tooltip={hasExternalChanges ? 'Reload to show new changes' : 'Reload file'}
                type='button'
              >
                <IconRefresh aria-hidden='true' size={14} />
                {hasExternalChanges ? <span aria-hidden='true' className='manage-file-change-indicator' /> : null}
              </ManageTooltipButton>
            )}
          </div>
        ) : isHtml ? (
          <div className='manage-preview-header-actions'>
            <ManageTooltipButton
              aria-label='Toggle annotations'
              aria-pressed={htmlAnnotationEnabled}
              className='manage-annotation-toggle'
              onClick={() => setHtmlAnnotationEnabled((current) => !current)}
              tooltip={htmlAnnotationEnabled ? 'Disable annotations' : 'Enable annotations'}
              type='button'
            >
              <IconMessagePlus aria-hidden='true' size={14} />
              <span>Annotate</span>
            </ManageTooltipButton>
            <ManageTooltipButton
              aria-label='Reload HTML file'
              className='manage-file-reload-button'
              onClick={onReload}
              tooltip='Reload HTML file'
              type='button'
            >
              <IconRefresh aria-hidden='true' size={14} />
            </ManageTooltipButton>
          </div>
        ) : null}
      </header>
      {!usesCompactArtifactHeader ? <div className='manage-preview-path'>{previewDisplayPath}</div> : null}
      {preview.kind === 'unsupported' ? (
        <ManagePreviewMessage
          icon={<IconAlertTriangle aria-hidden='true' size={21} />}
          title={preview.error ?? 'Preview unavailable'}
        />
      ) : isDrawing ? (
        <ManageExcalidrawEditor
          content={draftContent}
          fileName={preview.name}
          key={preview.path}
          onChange={onDraftContentChange}
        />
      ) : isHtml ? (
        <ManageHtmlRenderViewer
          annotationsEnabled={htmlAnnotationEnabled}
          content={draftContent}
          documentKey={preview.path}
          onOpenDocument={onOpenDocument}
        />
      ) : isMarkdown ? (
        <>
          <ManageMarkdownReviewViewer
            annotations={annotations}
            content={draftContent}
            documentKey={preview.path}
            gitBaseline={preview.gitBaseline}
            onContentChange={onDraftContentChange}
            onAnnotationPreviewChange={setAnnotationPreview}
            onSelectionClear={clearSelectedText}
            onSelectionCapture={captureSelectedText}
            onSelectionToolbarModeChange={setSelectionToolbarMode}
            selection={selection}
            selectionToolbarMode={selectionToolbarMode}
          />
          {selection && selectionToolbarMode === 'annotations' ? (
            <ManageAnnotationToolbar
              anchor={selection.anchor}
              onComment={openCommentForSelection}
              onDismiss={() => {
                setSelectionToolbarMode('annotations');
                setSelection(undefined);
              }}
              onFormatting={() => setSelectionToolbarMode('formatting')}
              onQuickLabel={addQuickLabel}
            />
          ) : null}
          {commentDraft ? (
            <ManageCommentPopover
              draft={commentDraft}
              onAddAttachmentFiles={addAttachmentFiles}
              onCancel={() => {
                setEditingAnnotationId(undefined);
                setCommentDraft(undefined);
              }}
              onDraftNoteChange={updateCommentDraftNote}
              onRemoveDraftAttachment={removeDraftAttachment}
              onSubmit={submitCommentDraft}
              submitLabel={editingAnnotationId ? 'Save' : 'Add'}
            />
          ) : null}
          {annotationPreview && !selection && !commentDraft ? (
            <ManageAnnotationPreviewCard onRemoveAnnotation={removePreviewAnnotation} preview={annotationPreview} />
          ) : null}
        </>
      ) : (
        <ManageTextEditor content={draftContent} language={language} onChange={onDraftContentChange} />
      )}
    </div>
  );
}

/**
 * The Send button always says where feedback will land before it is pressed:
 * the agent and session last clicked in the sidebar, or the clipboard when the
 * app has no session it can hand the text to.
 */
export function manageSendButtonLabel(
  sendState: ManageAnnotationSendState,
  sendTarget: ManageAnnotationSendTarget | null,
  counts: ManageAnnotationReviewCounts
): { text: string; tooltip: string } {
  switch (sendState.kind) {
    case 'sending':
      return { text: 'Sending', tooltip: 'Sending annotations' };
    case 'sent': {
      const where =
        sendState.delivery === 'chat'
          ? 'Added to chat'
          : sendState.delivery === 'terminal'
            ? 'Added to terminal'
            : 'Copied to clipboard';
      const across = sendState.fileCount > 1 ? ` across ${sendState.fileCount} files` : '';
      return {
        text: where,
        tooltip: `${where}: ${sendState.count} annotation${sendState.count === 1 ? '' : 's'}${across}`,
      };
    }
    case 'notice':
    case 'error':
      return { text: sendState.message, tooltip: sendState.message };
    case 'idle':
      break;
  }
  const resend = counts.pending === 0;
  const count = resend ? counts.sent : counts.pending;
  const what = resend ? `${count} again` : `${count} new`;
  const noun = `annotation${count === 1 ? '' : 's'}`;
  if (!sendTarget) {
    return {
      text: `Copy ${what}`,
      tooltip:
        count === 0
          ? 'No annotations to send'
          : `No agent session is selected in the sidebar, so the ${resend ? '' : 'new '}${noun} will be copied to the clipboard`,
    };
  }
  const surface = sendTarget.surface === 'chat' ? 'chat' : 'terminal';
  const verb = resend ? 'Resend' : 'Send';
  return {
    text: `${verb} ${what} \u25B8 ${sendTarget.agentLabel} in ${sendTarget.sessionTitle}`,
    tooltip:
      count === 0
        ? 'No annotations to send'
        : `${resend ? 'Add all' : 'Add'} ${count} ${resend ? '' : 'new '}${noun} ${resend ? 'again ' : ''}to the ${surface} of ${sendTarget.agentLabel} in ${sendTarget.sessionTitle} (${formatSidebarHotkeyLabel('cmd+enter')})`,
  };
}
