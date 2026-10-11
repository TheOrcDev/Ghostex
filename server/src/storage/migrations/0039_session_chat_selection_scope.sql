-- CDXC:SessionChat 2026-09-18 WHY:
-- A queued pick survives disconnects and restarts, so its scope has to survive with it.
-- 'default' is the value every pre-scope row already meant: change the agent's saved default.
ALTER TABLE session_chat_model_selections ADD COLUMN scope TEXT NOT NULL DEFAULT 'default';
PRAGMA user_version = 39;
