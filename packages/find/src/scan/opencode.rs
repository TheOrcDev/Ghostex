use std::fs;
use std::path::Path;

use crate::agent::Agent;

use super::*;

impl Scanner {
    // -----------------------------------------------------------------------
    // opencode
    // -----------------------------------------------------------------------

    pub(super) fn scan_opencode(&mut self) {
        let db = self.path(".local/share/opencode/opencode.db");
        self.scan_opencode_db(&db);
    }

    pub fn scan_opencode_db(&mut self, db_path: &Path) {
        if !db_path.exists() {
            return;
        }
        let fallback_ts = fs::metadata(db_path)
            .map(|m| mtime_seconds(&m))
            .unwrap_or(0);
        match read_opencode_rows(db_path) {
            Ok(rows) => {
                for row in rows {
                    self.parse_opencode_row(&row, fallback_ts);
                }
            }
            Err(err) => self.opencode_error = Some(err),
        }
    }

    /// Parse JSONL produced by `OPENCODE_QUERY` (one object per text part).
    pub fn parse_opencode(&mut self, data: &[u8]) {
        for line in data.split(|&b| b == b'\n') {
            if let Ok(text) = std::str::from_utf8(line) {
                self.parse_opencode_row(text, 0);
            }
        }
    }

    fn parse_opencode_row(&mut self, row: &str, fallback_ts: i64) {
        let Some(v) = parse_line(row.as_bytes()) else {
            return;
        };
        if string_field(&v, "role") != Some("user") {
            return;
        }
        if string_field(&v, "type") != Some("text") {
            return;
        }
        let Some(text) = string_field(&v, "text") else {
            return;
        };
        let Some(text) = visible_user_prompt(text) else {
            return;
        };
        let ts = timestamp_value(field(&v, "ts"));
        self.records.push(Record {
            agent: Agent::Opencode,
            title: title_from_object(&v).unwrap_or_default(),
            text,
            project: string_field(&v, "project").unwrap_or("").to_string(),
            session: string_field(&v, "session").unwrap_or("").to_string(),
            ts: if ts > 0 { ts } else { fallback_ts },
            meta: Meta {
                provider: string_field(&v, "provider").unwrap_or("").to_string(),
                model: string_field(&v, "model").unwrap_or("").to_string(),
                ..Default::default()
            },
        });
    }
}

const OPENCODE_QUERY: &str = concat!(
    "SELECT json_object(",
    "'role',json_extract(m.data,'$.role'),",
    "'type',json_extract(p.data,'$.type'),",
    "'text',json_extract(p.data,'$.text'),",
    "'title',null,",
    "'provider',json_extract(m.data,'$.model.providerID'),",
    "'model',json_extract(m.data,'$.model.modelID'),",
    "'project',s.directory,",
    "'session',s.id,",
    "'ts',coalesce(json_extract(m.data,'$.time.updated'),json_extract(m.data,'$.time.created'),json_extract(m.data,'$.updated_at'),json_extract(m.data,'$.created_at'),json_extract(m.data,'$.updatedAt'),json_extract(m.data,'$.createdAt'),json_extract(m.data,'$.timestamp'))) ",
    "FROM part p ",
    "JOIN message m ON p.message_id=m.id ",
    "JOIN session s ON p.session_id=s.id ",
    "WHERE json_extract(p.data,'$.type')='text';"
);

fn read_opencode_rows(db_path: &Path) -> Result<Vec<String>, String> {
    use rusqlite::{Connection, OpenFlags};
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("open opencode.db: {e}"))?;
    let mut stmt = conn
        .prepare(OPENCODE_QUERY)
        .map_err(|e| format!("prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        match row {
            Ok(text) => out.push(text),
            Err(e) => return Err(format!("row: {e}")),
        }
    }
    Ok(out)
}
