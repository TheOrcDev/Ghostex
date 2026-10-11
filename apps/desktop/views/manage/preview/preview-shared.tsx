import { type ReactNode } from 'react';

export function ManagePreviewMessage({ action, icon, title }: { action?: ReactNode; icon: ReactNode; title: string }) {
  return (
    <div className='manage-preview-message' data-has-action={action ? 'true' : undefined}>
      {icon}
      <span>{title}</span>
      {action}
    </div>
  );
}

export function isEditableEventTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) {
    return false;
  }
  if (target.matches("input, textarea, select, [contenteditable='true']")) {
    return true;
  }
  return Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
}
