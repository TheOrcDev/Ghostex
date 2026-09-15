import { useEffect, useId, useState, type ReactNode } from 'react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/packages/core-ui/app-tooltip';
import { writeTextToClipboard } from '../annotation-store';

/**
 * CDXC:Docs 2026-09-07 DECISION:
 * User: clicking the file name in the Docs top bar copies it and shows a tooltip.
 * Copy the displayed name or path, including the readable name of mounted folders.
 */
/**
 * CDXC:Docs 2026-09-15 DECISION:
 * User: unsaved changes are shown by the file icon in the top bar, which becomes a filled dot until the file is saved.
 */
export function ManageDocumentTitle({
  dirty = false,
  title,
  icon,
}: {
  dirty?: boolean;
  title: string;
  icon: ReactNode;
}) {
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'error'>('idle');
  const [tooltipOpen, setTooltipOpen] = useState(false);
  const triggerId = useId();

  useEffect(() => {
    if (copyState === 'idle') return;
    const timeout = window.setTimeout(() => {
      setCopyState('idle');
      setTooltipOpen(false);
    }, 1600);
    return () => window.clearTimeout(timeout);
  }, [copyState]);

  const copyTitle = async () => {
    try {
      await writeTextToClipboard(title);
      setCopyState('copied');
    } catch {
      setCopyState('error');
    }
    setTooltipOpen(true);
  };

  return (
    <Tooltip open={tooltipOpen} onOpenChange={setTooltipOpen} triggerId={triggerId}>
      <TooltipTrigger
        id={triggerId}
        closeOnClick={false}
        render={
          <button
            aria-label={dirty ? 'Unsaved changes. Copy file name' : 'Copy file name'}
            className='manage-preview-title'
            data-dirty={String(dirty)}
            onClick={() => void copyTitle()}
            type='button'
          >
            {dirty ? <span aria-hidden='true' className='manage-preview-title-unsaved' /> : icon}
            <span>
              <bdi>{title}</bdi>
            </span>
          </button>
        }
      />
      <TooltipContent align='start' alignOffset={20} side='bottom' sideOffset={8}>
        {copyState === 'copied'
          ? 'Copied!'
          : copyState === 'error'
            ? 'Could not copy file name'
            : dirty
              ? 'Unsaved changes. Press ⌘S or Ctrl+S to save. Click to copy file name'
              : 'Copy file name'}
      </TooltipContent>
    </Tooltip>
  );
}
