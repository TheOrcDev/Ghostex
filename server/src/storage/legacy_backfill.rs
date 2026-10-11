use super::*;

pub(super) fn backfill_legacy_macos_recent_projects(
    db: &mut Connection,
    paths: &GxserverPaths,
) -> Result<()> {
    /*
    CDXC:Projects 2026-06-27-19:37:
    Local Recent Projects are gxserver-owned shared project state. Existing users may already have matching P-project rows imported before the recent-project columns existed, while WK/macOS storage still carries `isRecentProject`. Reconcile that legacy file into `projects.isRecentProject` once so GPUI and macOS read the same drawer source without copying project names or paths into logs.
    */
    let existing_marker: Option<String> = db
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            [LEGACY_RECENT_PROJECTS_BACKFILL_METADATA_KEY],
            |row| row.get(0),
        )
        .optional()?;
    if existing_marker.is_some() {
        return Ok(());
    }

    let migrated_source_file = paths.app_state_dir.join(LEGACY_NATIVE_PROJECTS_STATE_FILE);
    let legacy_source_file = paths
        .home_dir
        .join(".ghostex")
        .join("state")
        .join(LEGACY_NATIVE_PROJECTS_STATE_FILE);
    let source_file = if migrated_source_file.is_file() {
        migrated_source_file
    } else {
        legacy_source_file
    };
    if !source_file.is_file() {
        write_recent_projects_backfill_marker(
            db,
            "skipped",
            Some("noLegacyState"),
            0,
            0,
            0,
            false,
        )?;
        return Ok(());
    }

    let source_text = match fs::read_to_string(&source_file) {
        Ok(text) => text,
        Err(_) => {
            write_recent_projects_backfill_marker(
                db,
                "skipped",
                Some("unreadableLegacyState"),
                0,
                0,
                0,
                true,
            )?;
            return Ok(());
        }
    };
    let parsed: Value = match serde_json::from_str(&source_text) {
        Ok(value) => value,
        Err(_) => {
            write_recent_projects_backfill_marker(
                db,
                "skipped",
                Some("malformedLegacyState"),
                0,
                0,
                0,
                true,
            )?;
            return Ok(());
        }
    };
    let recent_projects = parsed
        .get("projects")
        .and_then(Value::as_array)
        .map(|projects| {
            projects
                .iter()
                .filter(|project| {
                    project.get("isRecentProject").and_then(Value::as_bool) == Some(true)
                })
                .filter_map(|project| {
                    let project_id = project
                        .get("projectId")
                        .and_then(Value::as_str)?
                        .trim()
                        .to_string();
                    if project_id.is_empty() {
                        return None;
                    }
                    let recent_closed_at = project
                        .get("recentClosedAt")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
                        .map(str::to_string);
                    Some((project_id, recent_closed_at))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if recent_projects.is_empty() {
        write_recent_projects_backfill_marker(
            db,
            "completed",
            Some("noRecentProjects"),
            0,
            0,
            0,
            true,
        )?;
        return Ok(());
    }

    let transaction = db.transaction()?;
    let mut matched_projects = 0usize;
    let mut updated_projects = 0usize;
    for (project_id, recent_closed_at) in &recent_projects {
        let matched: i64 = transaction.query_row(
            r#"
            SELECT COUNT(*)
            FROM projects
            WHERE projectId = ?1
              AND path IS NOT NULL
              AND trim(path) <> ''
            "#,
            [project_id],
            |row| row.get(0),
        )?;
        if matched <= 0 {
            continue;
        }
        matched_projects += 1;
        updated_projects += transaction.execute(
            r#"
            UPDATE projects
            SET isRecentProject = 1,
                recentClosedAt = COALESCE(?2, recentClosedAt, updatedAt)
            WHERE projectId = ?1
              AND path IS NOT NULL
              AND trim(path) <> ''
              AND isRecentProject = 0
            "#,
            params![project_id, recent_closed_at],
        )?;
    }
    write_recent_projects_backfill_marker_in_transaction(
        &transaction,
        "completed",
        None,
        recent_projects.len(),
        matched_projects,
        updated_projects,
        true,
    )?;
    transaction.commit()?;
    Ok(())
}

fn write_recent_projects_backfill_marker(
    db: &Connection,
    status: &str,
    skipped_reason: Option<&str>,
    legacy_recent_projects: usize,
    matched_projects: usize,
    updated_projects: usize,
    source_file_read: bool,
) -> Result<()> {
    write_recent_projects_backfill_marker_in_connection(
        db,
        status,
        skipped_reason,
        legacy_recent_projects,
        matched_projects,
        updated_projects,
        source_file_read,
    )
}

fn write_recent_projects_backfill_marker_in_connection(
    db: &Connection,
    status: &str,
    skipped_reason: Option<&str>,
    legacy_recent_projects: usize,
    matched_projects: usize,
    updated_projects: usize,
    source_file_read: bool,
) -> Result<()> {
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let marker = create_recent_projects_backfill_marker(
        &timestamp,
        status,
        skipped_reason,
        legacy_recent_projects,
        matched_projects,
        updated_projects,
        source_file_read,
    );
    db.execute(
        r#"
        INSERT INTO metadata (key, value, updatedAt)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
          value = excluded.value,
          updatedAt = excluded.updatedAt
        "#,
        params![
            LEGACY_RECENT_PROJECTS_BACKFILL_METADATA_KEY,
            marker.to_string(),
            timestamp,
        ],
    )?;
    Ok(())
}

fn write_recent_projects_backfill_marker_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    status: &str,
    skipped_reason: Option<&str>,
    legacy_recent_projects: usize,
    matched_projects: usize,
    updated_projects: usize,
    source_file_read: bool,
) -> Result<()> {
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let marker = create_recent_projects_backfill_marker(
        &timestamp,
        status,
        skipped_reason,
        legacy_recent_projects,
        matched_projects,
        updated_projects,
        source_file_read,
    );
    transaction.execute(
        r#"
        INSERT INTO metadata (key, value, updatedAt)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
          value = excluded.value,
          updatedAt = excluded.updatedAt
        "#,
        params![
            LEGACY_RECENT_PROJECTS_BACKFILL_METADATA_KEY,
            marker.to_string(),
            timestamp,
        ],
    )?;
    Ok(())
}

fn create_recent_projects_backfill_marker(
    timestamp: &str,
    status: &str,
    skipped_reason: Option<&str>,
    legacy_recent_projects: usize,
    matched_projects: usize,
    updated_projects: usize,
    source_file_read: bool,
) -> Value {
    let mut marker = json!({
        "completedAt": timestamp,
        "id": LEGACY_RECENT_PROJECTS_BACKFILL_ID,
        "legacyRecentProjects": legacy_recent_projects,
        "matchedProjects": matched_projects,
        "sourceFilesRead": if source_file_read {
            vec![LEGACY_NATIVE_PROJECTS_STATE_FILE]
        } else {
            Vec::<&str>::new()
        },
        "status": status,
        "updatedProjects": updated_projects,
    });
    if let Some(skipped_reason) = skipped_reason {
        marker["skippedReason"] = json!(skipped_reason);
    }
    marker
}
