use crate::*;
use std::time::Duration;

/// Native chat views kept warm for one project, counting the ones the user opened.
const NATIVE_CHAT_WARM_VIEWS_PER_PROJECT: usize = 5;
/// CDXC:SessionChat 2026-09-19 DECISION:
/// User: keep more chat views in memory; clicking through four sessions and back must not show a skeleton again. A pooled-out view boots from scratch, so the app-wide pool is large and hidden views pause their subscription instead of being dropped.
pub(crate) const NATIVE_CHAT_WARM_VIEWS_TOTAL: usize = 24;

impl GhostexGpuiApp {
    /// CDXC:SessionChat 2026-09-19 WHY:
    /// A session's first chat open cost about 300ms after the click (runtime boot, two service-thread round trips, transcript projection) while an already warm view painted in about 25ms; Waku feels instant because every transcript is in memory before the click.
    /// The active project's most recently used chat-eligible tab sessions get their native chat views created in the background, one every 150ms, so a click on a session the user actually switches between lands on a warm view. Recency comes from the sidebar's last interaction time; the first ten tabs in workspace order, which this supersedes, warmed sessions nobody was going to open. Views never show until focused, and each project keeps at most five warm views unless the user opened more.
    pub(crate) fn schedule_native_chat_prewarm(&mut self, cx: &mut gpui::Context<Self>) {
        self.ensure_native_chat_pool_pass(cx);
        if self.agents_chat_prewarm_scheduled {
            return;
        }
        let pending = self.native_chat_prewarm_candidates();
        if pending.is_empty() {
            return;
        }
        self.agents_chat_prewarm_scheduled = true;
        cx.spawn(async move |this, cx| {
            for session_id in pending {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                let keep = this
                    .update(cx, |this, cx| {
                        if this.native_chat_views.len() >= NATIVE_CHAT_WARM_VIEWS_PER_PROJECT
                            || this.native_chat_views_total() >= NATIVE_CHAT_WARM_VIEWS_TOTAL
                        {
                            return false;
                        }
                        if this.native_chat_views.contains_key(&session_id)
                            || !this
                                .agents_workspace
                                .terminal_session_ids()
                                .contains(&session_id)
                        {
                            return true;
                        }
                        this.ensure_native_chat(session_id, cx);
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
            let _ = this.update(cx, |this, _| this.agents_chat_prewarm_scheduled = false);
        })
        .detach();
    }

    /// The active project's chat-eligible tab sessions without a warm view, most recently used first.
    fn native_chat_prewarm_candidates(&self) -> Vec<TerminalSessionId> {
        let room = NATIVE_CHAT_WARM_VIEWS_PER_PROJECT
            .saturating_sub(self.native_chat_views.len())
            .min(NATIVE_CHAT_WARM_VIEWS_TOTAL.saturating_sub(self.native_chat_views_total()));
        if room == 0 {
            return Vec::new();
        }
        let mut candidates = self
            .agents_workspace
            .terminal_session_ids()
            .into_iter()
            .filter(|session_id| !self.native_chat_views.contains_key(session_id))
            .filter(|session_id| {
                self.agents_chat_mode_sessions.contains(session_id)
                    || self.agents_session_chat_eligible(*session_id)
            })
            .enumerate()
            .map(|(order, session_id)| {
                (
                    self.native_chat_last_interaction(session_id),
                    order,
                    session_id,
                )
            })
            .collect::<Vec<_>>();
        // ISO-8601 UTC stamps order lexically; sessions without one come last, in workspace order.
        candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        candidates
            .into_iter()
            .map(|(_, _, session_id)| session_id)
            .take(room)
            .collect()
    }

    fn native_chat_last_interaction(&self, session_id: TerminalSessionId) -> Option<String> {
        let sidebar_session_id = match self.workspace_terminal_key_for_shell_session(session_id)? {
            GpuiWorkspaceTerminalSessionKey::Local(key) => {
                gpui_combined_presentation_session_id(&key.project_id, &key.session_id)
            }
            GpuiWorkspaceTerminalSessionKey::Remote(key) => gpui_remote_scoped_session_id(
                &key.remote_machine_id,
                &key.project_id,
                &key.session_id,
            ),
        };
        self.native_sidebar
            .snapshot
            .as_ref()?
            .groups
            .iter()
            .flat_map(|group| group.sessions.iter())
            .find(|session| session.session_id == sidebar_session_id)
            .and_then(|session| session.last_interaction_at.clone())
    }
}
