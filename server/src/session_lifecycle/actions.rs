use super::*;

/*
Guards. These are the server-side twins of `canSettleSidebarV2Session`: a stale
or raced client must not park work that is blocked on the user (attention) or
in motion (working) behind a settle.

CDXC:Sessions 2026-09-12 DECISION:
User: Snooze defers any session for 1 hour, 3 hours, tomorrow or next week and puts it to sleep. A session waiting on
the user is exactly what people snooze ("deal with this tomorrow"), so snooze has no activity guard at all; only the
wake time is validated.
*/
fn reject_when_settle_is_blocked(
    session: &Value,
    now_iso: &str,
    session_ref: &str,
) -> Result<(), DomainStateError> {
    match session_activity(session, now_iso).as_str() {
        "attention" => Err(DomainStateError::bad_request(format!(
            "Session {session_ref} is waiting on you and cannot be settled."
        ))),
        "working" => Err(DomainStateError::bad_request(format!(
            "Session {session_ref} is working and cannot be settled."
        ))),
        _ => Ok(()),
    }
}

fn require_session(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
) -> Result<Value, DomainStateError> {
    repository
        .get_session(project_id, session_id)?
        .ok_or_else(|| {
            DomainStateError::not_found(format!(
                "Session {project_id}/{session_id} does not exist."
            ))
        })
}

fn apply(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
    session: Value,
    current: SessionLifecycleFields,
    next: SessionLifecycleFields,
    now_iso: &str,
) -> Result<SessionLifecycleOutcome, DomainStateError> {
    if next == current {
        return Ok(SessionLifecycleOutcome {
            changed: false,
            session,
        });
    }
    let session = repository.write_session_lifecycle(project_id, session_id, &next, now_iso)?;
    Ok(SessionLifecycleOutcome {
        changed: true,
        session,
    })
}

/*
Settle. Settling an already-settled session keeps the original `settledAt` so a
double click or a bulk settle is a silent no-op instead of an error or a
reordering churn from redundant settle emissions. Settling a session the
sweep auto-settled DOES stamp `settledAt`: that is the user promoting an
automatic decision into an explicit one, which treats a row
with a null `settledAt` as not-yet-explicitly-settled.
*/
pub fn settle_session(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
    now_iso: &str,
) -> Result<SessionLifecycleOutcome, DomainStateError> {
    let session = require_session(repository, project_id, session_id)?;
    let session_ref = format!("{project_id}/{session_id}");
    reject_when_settle_is_blocked(&session, now_iso, &session_ref)?;
    let current = SessionLifecycleFields::from_session(&session);
    let mut next = current.clone();
    if !(current.is_settled_override() && current.settled_at.is_some()) {
        next.settled_override = Some("settled".to_string());
        next.settled_at = Some(now_iso.to_string());
        next.settled_override_at = Some(now_iso.to_string());
    }
    apply(
        repository, project_id, session_id, session, current, next, now_iso,
    )
}

/*
Un-settle. The "active" pin is the user saying "keep this in my inbox": it
suppresses auto-settle until real activity outruns the pin's stamp, at which
point the sweep clears it and the ordinary rules apply again. This is the
`thread.unsettled` with `reason: "user"` projected to `settledOverride: "active"`
plus its `reason: "activity"` reset, expressed against gxserver's activity clock
instead of an event log.
*/
pub fn unsettle_session(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
    now_iso: &str,
) -> Result<SessionLifecycleOutcome, DomainStateError> {
    let session = require_session(repository, project_id, session_id)?;
    let current = SessionLifecycleFields::from_session(&session);
    let mut next = current.clone();
    if !current.is_active_override() {
        next.settled_override = Some("active".to_string());
        next.settled_at = None;
        next.settled_override_at = Some(now_iso.to_string());
    }
    apply(
        repository, project_id, session_id, session, current, next, now_iso,
    )
}

/*
Snooze. A wake time in the past (or an unparseable one) would produce a session
that is snoozed and awake at once — the row would never leave the inbox but
would still carry snooze state — so it is rejected rather than silently
normalized. Re-snoozing to the same wake time keeps the original `snoozedAt` so
duplicates stay no-ops; a different wake time is a real change and stamps fresh.
*/
pub fn snooze_session(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
    snoozed_until: &str,
    now_iso: &str,
) -> Result<SessionLifecycleOutcome, DomainStateError> {
    let session = require_session(repository, project_id, session_id)?;
    let now_ms = parse_iso_ms(now_iso).ok_or_else(|| {
        DomainStateError::corrupt_state(format!("Invalid gxserver timestamp: {now_iso}."))
    })?;
    let wake_at_ms = parse_iso_ms(snoozed_until.trim()).filter(|wake_at_ms| *wake_at_ms > now_ms);
    let Some(wake_at_ms) = wake_at_ms else {
        return Err(DomainStateError::bad_request(format!(
            "snoozedUntil must be an ISO timestamp in the future; got {}.",
            snoozed_until.trim()
        )));
    };
    let normalized_until = iso_from_ms(wake_at_ms);
    let current = SessionLifecycleFields::from_session(&session);
    let mut next = current.clone();
    let is_duplicate = current.snoozed_until.as_deref() == Some(normalized_until.as_str())
        && current.snoozed_at.is_some();
    if !is_duplicate {
        next.snoozed_until = Some(normalized_until);
        next.snoozed_at = Some(now_iso.to_string());
    }
    apply(
        repository, project_id, session_id, session, current, next, now_iso,
    )
}

/// Wake. Clears both snooze fields; waking a session that is not snoozed is a
/// no-op rather than an error.
pub fn unsnooze_session(
    repository: &DomainRepository<'_>,
    project_id: &str,
    session_id: &str,
    now_iso: &str,
) -> Result<SessionLifecycleOutcome, DomainStateError> {
    let session = require_session(repository, project_id, session_id)?;
    let current = SessionLifecycleFields::from_session(&session);
    let mut next = current.clone();
    next.clear_snooze();
    apply(
        repository, project_id, session_id, session, current, next, now_iso,
    )
}
