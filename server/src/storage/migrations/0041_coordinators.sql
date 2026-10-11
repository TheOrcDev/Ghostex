-- CDXC:Coordinators 2026-09-30 WHY:
-- A coordinator is an agent session with a role, and a thread is a session it started. Both are
-- side tables keyed by (projectId, sessionId) instead of columns on `sessions`: a table rebuild of
-- `sessions` cascade-deletes `delayed_sends` (see 0036), and neither row may disappear because a
-- session row was pruned, so there is no foreign key either (like `session_agent_notes`).
-- `memoryJson` is the coordinator's numbered notes, `[{ "text", "createdAt" }]`.
-- The thread columns after `resolvedAt` are the supervisor's bookkeeping: it reports a finished turn
-- only after it saw the thread work (`observedWorking`), and a question only once per prompt
-- (`reportedPromptKey`); both are written after the report was delivered, so a restart never loses
-- or repeats one.
CREATE TABLE IF NOT EXISTS coordinators (
  projectId TEXT NOT NULL,
  sessionId TEXT NOT NULL,
  goal TEXT NOT NULL DEFAULT '',
  instructions TEXT NOT NULL DEFAULT '',
  memoryJson TEXT NOT NULL DEFAULT '[]',
  createdAt TEXT NOT NULL,
  updatedAt TEXT NOT NULL,
  PRIMARY KEY (projectId, sessionId)
);

CREATE TABLE IF NOT EXISTS coordinator_threads (
  projectId TEXT NOT NULL,
  sessionId TEXT NOT NULL,
  coordinatorProjectId TEXT NOT NULL,
  coordinatorSessionId TEXT NOT NULL,
  task TEXT NOT NULL DEFAULT '',
  resolvedAt TEXT,
  observedWorking INTEGER NOT NULL DEFAULT 0 CHECK (observedWorking IN (0, 1)),
  reportedAt TEXT,
  reportedPromptKey TEXT,
  lastReport TEXT,
  createdAt TEXT NOT NULL,
  updatedAt TEXT NOT NULL,
  PRIMARY KEY (projectId, sessionId)
);

CREATE INDEX IF NOT EXISTS idx_coordinator_threads_coordinator
  ON coordinator_threads(coordinatorProjectId, coordinatorSessionId);

PRAGMA user_version = 41;
