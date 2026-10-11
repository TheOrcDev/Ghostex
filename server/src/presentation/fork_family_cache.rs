use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

use rusqlite::Connection;

use super::SessionForkFamilies;
use crate::domain::{sql_error, DomainRepository, DomainStateError};

const MAX_DATABASES: usize = 4;
const MAX_CACHED_SESSIONS: usize = 16_384;

struct CachedFamilies {
    database: String,
    server_id: String,
    revision: String,
    families: Arc<SessionForkFamilies>,
}

/// CDXC:StateSync 2026-09-13 WHY:
/// A presentation delta formerly extracted every session's JSON and rebuilt the fork graph, even for a title-only update.
/// Read the durable lineage token and rows in one SQLite snapshot, and never publish a caller's uncommitted graph into the shared cache.
pub(crate) fn read_session_fork_families(
    db: &Connection,
    server_id: &str,
) -> Result<Arc<SessionForkFamilies>, DomainStateError> {
    let database = db.path().filter(|path| !path.is_empty());
    if !db.is_autocommit() || database.is_none() {
        return Ok(Arc::new(SessionForkFamilies::build(
            &DomainRepository::new(db, server_id).list_session_fork_rows()?,
        )));
    }
    static CACHE: OnceLock<Mutex<VecDeque<CachedFamilies>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let transaction = db.unchecked_transaction().map_err(sql_error)?;
    let revision: String = transaction
        .query_row(
            "SELECT value FROM metadata WHERE key = 'sessionForkRevision'",
            [],
            |row| row.get(0),
        )
        .map_err(sql_error)?;
    /*
    CDXC:StateSync 2026-09-18 WHY:
    Legacy registries carry the same sessionId under several projects, and the builder picks among such rows by updatedAt order, which a title write can change without touching the lineage token.
    The cache used to refuse those registries outright, so one with 50 duplicates rebuilt the whole graph from 9,000 rows on every delta (over half of gxserver's CPU). The duplicates' own order joins the key instead; it is a few rows to read and changes exactly when the pick could.
    */
    let revision = format!("{revision}|{}", duplicate_session_order(&transaction)?);
    let database = database.unwrap();
    let cached = {
        let mut cache = cache
            .lock()
            .map_err(|_| DomainStateError::corrupt_state("Session fork cache is poisoned."))?;
        cache
            .iter()
            .position(|entry| {
                entry.database == database
                    && entry.server_id == server_id
                    && entry.revision == revision
            })
            .map(|index| {
                let entry = cache.remove(index).unwrap();
                let families = entry.families.clone();
                cache.push_back(entry);
                families
            })
    };
    if let Some(families) = cached {
        transaction.commit().map_err(sql_error)?;
        return Ok(families);
    }
    let rows = DomainRepository::new(&transaction, server_id).list_session_fork_rows()?;
    let families = Arc::new(SessionForkFamilies::build(&rows));
    transaction.commit().map_err(sql_error)?;
    if rows.len() <= MAX_CACHED_SESSIONS {
        let mut cache = cache
            .lock()
            .map_err(|_| DomainStateError::corrupt_state("Session fork cache is poisoned."))?;
        cache.retain(|entry| entry.database != database || entry.server_id != server_id);
        if cache.len() >= MAX_DATABASES {
            cache.pop_front();
        }
        cache.push_back(CachedFamilies {
            database: database.to_owned(),
            server_id: server_id.to_owned(),
            revision,
            families: families.clone(),
        });
    }
    Ok(families)
}

/// The ordered rows of every sessionId that appears more than once, which is what decides the builder's pick among them.
fn duplicate_session_order(db: &Connection) -> Result<String, DomainStateError> {
    let mut statement = db
        .prepare(
            "SELECT sessionId, projectId FROM sessions
             WHERE sessionId IN (SELECT sessionId FROM sessions GROUP BY sessionId HAVING count(*) > 1)
             ORDER BY updatedAt DESC, projectId ASC, sessionId ASC",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_error)?;
    let mut order = String::new();
    for row in rows {
        let (session_id, project_id) = row.map_err(sql_error)?;
        order.push_str(&session_id);
        order.push('/');
        order.push_str(&project_id);
        order.push(',');
    }
    Ok(order)
}
