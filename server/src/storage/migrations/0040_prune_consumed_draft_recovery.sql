-- CDXC:Drafts 2026-09-19 WHY:
-- Consumed revisions were hidden from recovery but never deleted, so the table grew to tens of thousands of rows that nothing could ever show again. consume_in now deletes them as they are consumed; this removes the backlog. Unsent revisions are untouched.
DELETE FROM session_chat_draft_recovery
WHERE EXISTS (
  SELECT 1 FROM session_chat_draft_versions v
  WHERE v.projectId = session_chat_draft_recovery.projectId
    AND v.sessionId = session_chat_draft_recovery.sessionId
    AND v.draftId = session_chat_draft_recovery.draftId
    AND session_chat_draft_recovery.revision <= v.consumed
);
PRAGMA user_version = 40;
