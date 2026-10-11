use crate::*;

impl GhostexGpuiApp {
    pub(crate) fn begin_session_chat_native_request(
        &mut self,
        session_id: TerminalSessionId,
    ) -> Option<u64> {
        let state = self.agents_chat_page_states.get_mut(&session_id)?;
        state.pending_native_requests += 1;
        Some(state.generation)
    }

    pub(crate) fn dispatch_session_chat_generation_response(
        &mut self,
        generation: u64,
        callback: &str,
        payload: &serde_json::Value,
        finish_request: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(view) = self.native_chat_for_generation(generation) else {
            return;
        };
        if finish_request {
            for state in self.agents_chat_page_states.values_mut().chain(
                self.parked_agents_chat_runtimes_by_project
                    .values_mut()
                    .flat_map(|parked| parked.page_states.values_mut()),
            ) {
                if state.generation == generation {
                    state.pending_native_requests = state.pending_native_requests.saturating_sub(1);
                    break;
                }
            }
        }
        view.update(cx, |view, cx| view.receive_callback(callback, payload, cx));
    }
}
