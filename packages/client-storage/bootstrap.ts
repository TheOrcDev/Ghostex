import { initializeClientStorage, installBrowserGuard } from './service';

if (process.env.NODE_ENV !== 'production') installBrowserGuard();

/** A failed hydration must not mount an empty editor over an existing unsaved draft. */
export function bootClientStorage(
  start: () => void | Promise<void>,
  element: HTMLElement | null = document.getElementById('root')
): void {
  void initializeClientStorage()
    .then(start)
    .catch((error: unknown) => {
      console.error('[client-storage] Could not open persistent storage.', error);
      if (!element) return;
      const message = document.createElement('p');
      message.textContent = 'Your saved data could not be loaded. Retry to open this view.';
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.textContent = 'Retry loading';
      retry.onclick = () => {
        element.replaceChildren();
        bootClientStorage(start, element);
      };
      element.replaceChildren(message, retry);
    });
}
