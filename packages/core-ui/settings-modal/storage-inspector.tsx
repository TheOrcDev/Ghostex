import { useEffect, useState } from 'react';
import {
  clearDisposableStorage,
  initializeClientStorage,
  inspectClientStorage,
  retryClientStorage,
  subscribeStorage,
  type StoreId,
} from '@/packages/client-storage';
import { Button } from '@/packages/components/ui/button';

function size(bytes: number): string {
  return bytes >= 1024 * 1024 ? `${(bytes / (1024 * 1024)).toFixed(1)} MiB` : `${(bytes / 1024).toFixed(1)} KiB`;
}

export function StorageInspector() {
  const [snapshot, setSnapshot] = useState(inspectClientStorage);
  const [error, setError] = useState('');
  useEffect(() => {
    const refresh = () => setSnapshot(inspectClientStorage());
    void initializeClientStorage()
      .then(refresh)
      .catch((error: unknown) => setError(String(error)));
    const unsubscribe = subscribeStorage(refresh);
    const timer = setInterval(refresh, 2_000);
    return () => {
      unsubscribe();
      clearInterval(timer);
    };
  }, []);
  return (
    <div className='grid min-w-0 gap-3 rounded-lg border border-border p-4' data-storage-inspector>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <div>
          <h3 className='text-sm font-medium'>Storage usage</h3>
          <p className='text-xs text-muted-foreground'>
            Space used by this browser profile. Sizes include keys and serialized values.
          </p>
        </div>
        <Button size='sm' variant='outline' onClick={retryClientStorage}>
          Retry pending saves
        </Button>
      </div>
      <div className='grid gap-2 sm:grid-cols-3'>
        {Object.entries(snapshot.totals).map(([backend, total]) => (
          <div key={backend} className='rounded-md bg-muted/40 p-2 text-xs'>
            <div className='font-medium'>
              {backend === 'indexeddb' ? 'IndexedDB' : backend === 'local' ? 'Local preferences' : 'Temporary storage'}
            </div>
            <div className={total.bytes > total.budget ? 'text-destructive' : 'text-muted-foreground'}>
              {size(total.bytes)} / {size(total.budget)}
            </div>
          </div>
        ))}
      </div>
      {error ? (
        <p role='alert' className='text-xs text-destructive'>
          {error}
        </p>
      ) : null}
      <div
        className='max-h-96 min-h-0 overflow-auto rounded-md border border-border'
        tabIndex={0}
        aria-label='Storage owners'
      >
        {[...snapshot.stores]
          .sort((a, b) => b.bytes - a.bytes)
          .map((store) => (
            <details key={store.id} className='border-b border-border p-2 last:border-b-0'>
              <summary className='cursor-pointer text-xs'>
                <span className='font-medium'>{store.owner}</span>
                <span className='ml-2 text-muted-foreground'>
                  {size(store.bytes)} / {size(store.maxBytes)} · {store.entries} entries
                </span>
                {store.pending ? (
                  <span className='ml-2'>
                    {store.pending} pending ({size(store.pendingBytes)})
                  </span>
                ) : null}
                {store.lastFailure ? <span className='ml-2 text-destructive'>Save failure recorded</span> : null}
              </summary>
              <div className='mt-2 grid gap-1 break-words text-xs text-muted-foreground'>
                <p>
                  {store.backend} · {store.policy} · {store.schema}
                </p>
                <p>
                  {store.writesPerMinute} writes in the last minute · limit {store.maxEntries} entries
                </p>
                <p>Owner: {store.source}</p>
                {store.lastFailure ? (
                  <p>
                    Last failure: {store.lastFailure.reason} at {new Date(store.lastFailure.at).toLocaleTimeString()}
                  </p>
                ) : null}
                {store.largest.map((entry) => (
                  <p key={entry.key} className='break-all'>
                    {size(entry.bytes)} · {entry.key}
                  </p>
                ))}
                {store.policy === 'cache' && store.entries > 0 ? (
                  <Button
                    className='mt-1 w-fit'
                    size='sm'
                    variant='outline'
                    onClick={() => {
                      setError('');
                      void clearDisposableStorage(store.id as StoreId).catch((error: unknown) =>
                        setError(String(error))
                      );
                    }}
                  >
                    Clear this cache
                  </Button>
                ) : null}
              </div>
            </details>
          ))}
      </div>
      {snapshot.unknown.length ? (
        <details className='text-xs'>
          <summary className='cursor-pointer text-destructive'>
            {snapshot.unknown.length} unregistered or unmigrated entries
          </summary>
          <div className='mt-2 max-h-40 overflow-auto break-all' tabIndex={0}>
            {snapshot.unknown.map((entry) => (
              <p key={`${entry.backend}:${entry.key}`}>
                {entry.backend} · {size(entry.bytes)} · {entry.key}
              </p>
            ))}
          </div>
        </details>
      ) : null}
      <p className='text-xs text-muted-foreground'>
        Only disposable caches can be cleared here. Drafts and pending work are protected. Diagnostic history is kept in
        memory.
      </p>
    </div>
  );
}
