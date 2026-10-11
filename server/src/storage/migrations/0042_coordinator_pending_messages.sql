-- CDXC:Coordinators 2026-10-04 WHY:
-- A message a coordinator hands to its thread (a start-thread brief, a follow-up) is watched until
-- the thread's transcript records it: `pendingMessage` is the start of the message and
-- `pendingMessageAt` when it was sent. The supervisor clears both once the transcript shows it, or
-- after it told the coordinator the message did not arrive, so a restart never loses the watch.
ALTER TABLE coordinator_threads ADD COLUMN pendingMessage TEXT;
ALTER TABLE coordinator_threads ADD COLUMN pendingMessageAt TEXT;

PRAGMA user_version = 42;
