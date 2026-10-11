use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::domain::DomainStateError;

type SessionKey = (String, String);
static SWITCHES: OnceLock<Mutex<HashSet<SessionKey>>> = OnceLock::new();

fn switches() -> &'static Mutex<HashSet<SessionKey>> {
    SWITCHES.get_or_init(Default::default)
}

/// CDXC:Drafts 2026-09-16 WHY:
/// The draft row changes before its old CLI exits. Live process discovery could adopt that old CLI again, clearing the newly selected account and launch plan on each family change.
/// Keep identity observations out of this handoff until the serialized exit and launch writes finish, including cancellation and failure.
pub(crate) struct DraftAgentSwitch(SessionKey);

impl DraftAgentSwitch {
    pub(crate) fn begin(project_id: &str, session_id: &str) -> Result<Self, DomainStateError> {
        let key = (project_id.to_string(), session_id.to_string());
        if !switches().lock().unwrap().insert(key.clone()) {
            return Err(DomainStateError::bad_request(
                "This draft is already switching agents. Wait for the switch to finish.",
            ));
        }
        Ok(Self(key))
    }

    pub(crate) fn finish_after(
        self,
        completion: tokio::sync::oneshot::Receiver<
            Result<(), crate::session_chat_send::SessionChatSendError>,
        >,
    ) {
        tokio::spawn(async move {
            let _guard = self;
            let _ = completion.await;
        });
    }
}

impl Drop for DraftAgentSwitch {
    fn drop(&mut self) {
        // The presentation probe caches the old process for two seconds.
        crate::zmx::invalidate_zmx_process_identity_cache();
        switches().lock().unwrap().remove(&self.0);
    }
}

pub(crate) fn draft_agent_switch_in_progress(project_id: &str, session_id: &str) -> bool {
    switches()
        .lock()
        .unwrap()
        .contains(&(project_id.to_string(), session_id.to_string()))
}
