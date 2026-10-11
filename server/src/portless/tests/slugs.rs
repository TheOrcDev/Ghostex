use rusqlite::params;

use super::*;
use crate::{
    paths::get_gxserver_paths,
    storage::{initialize_gxserver_storage, open_gxserver_database},
};

#[test]
fn project_slug_create_read_update_round_trip() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "P1main", "First Display Name");
    let repository = PortlessRepository::new(&db);

    let created = repository
        .upsert_project_slug("P1main", "ghostex")
        .expect("create project slug");
    assert_eq!(created.project_id, "P1main");
    assert_eq!(created.slug, "ghostex");
    assert_eq!(
        repository
            .read_project_slug("P1main")
            .expect("read project slug")
            .map(|record| record.slug),
        Some("ghostex".to_string())
    );

    db.execute(
        "UPDATE projects SET name = ?2, updatedAt = ?3 WHERE projectId = ?1",
        params!["P1main", "Renamed Display Name", "2026-06-22T18:41:00.000Z"],
    )
    .expect("rename display name");
    assert_eq!(
        repository
            .read_project_slug("P1main")
            .expect("read after display rename")
            .map(|record| record.slug),
        Some("ghostex".to_string())
    );

    let updated = repository
        .upsert_project_slug("P1main", "ghostex-app")
        .expect("update project slug");
    assert_eq!(updated.slug, "ghostex-app");
    assert_eq!(
        repository
            .read_project_slug("P1main")
            .expect("read updated project slug")
            .map(|record| record.slug),
        Some("ghostex-app".to_string())
    );
}

#[test]
fn worktree_slug_create_read_update_is_separate_from_display_names() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project(&db, "P2main", "Main Display Name");
    let repository = PortlessRepository::new(&db);

    let created = repository
        .upsert_worktree_slug("P2main", "P2wtfix", "fix-ui")
        .expect("create worktree slug");
    assert_eq!(created.project_id, "P2main");
    assert_eq!(created.worktree_key, "P2wtfix");
    assert_eq!(created.slug, "fix-ui");

    db.execute(
        "UPDATE projects SET name = ?2, updatedAt = ?3 WHERE projectId = ?1",
        params![
            "P2main",
            "Completely Different Display Name",
            "2026-06-22T18:42:00.000Z"
        ],
    )
    .expect("rename project display name");
    assert_eq!(
        repository
            .read_worktree_slug("P2main", "P2wtfix")
            .expect("read worktree slug")
            .map(|record| record.slug),
        Some("fix-ui".to_string())
    );

    let updated = repository
        .upsert_worktree_slug("P2main", "P2wtfix", "fix-ui-2")
        .expect("update worktree slug");
    assert_eq!(updated.slug, "fix-ui-2");
    assert_eq!(
        repository
            .read_worktree_slug("P2main", "P2wtfix")
            .expect("read updated worktree slug")
            .map(|record| record.slug),
        Some("fix-ui-2".to_string())
    );
}

#[test]
fn ensure_project_slug_persists_first_assignment_across_project_renames() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "P1main",
        "First Display Name",
        Some("/tmp/first-display-name"),
        "2026-06-22T18:41:00.000Z",
    );
    let repository = PortlessRepository::new(&db);

    let created = repository
        .ensure_project_slug("P1main")
        .expect("ensure project slug");
    assert_eq!(created.slug, "first-display-name");

    db.execute(
        "UPDATE projects SET name = ?2, path = ?3, updatedAt = ?4 WHERE projectId = ?1",
        params![
            "P1main",
            "Renamed Project",
            "/tmp/renamed-project",
            "2026-06-22T18:45:00.000Z"
        ],
    )
    .expect("rename project");
    assert_eq!(
        repository
            .ensure_project_slug("P1main")
            .expect("ensure after rename")
            .slug,
        "first-display-name"
    );
    assert_eq!(
        repository
            .backfill_domain_identities()
            .expect("repeat backfill")
            .projects
            .into_iter()
            .find(|project| project.project_id == "P1main")
            .map(|project| project.slug),
        Some("first-display-name".to_string())
    );
}

#[test]
fn project_slug_uses_path_basename_when_visible_identity_has_no_label() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Ppath",
        "!!!",
        Some("/Users/person/dev/Path Fallback App"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_project_with_path(&db, "Pempty", "!!!", None, "2026-06-22T18:42:00.000Z");
    let repository = PortlessRepository::new(&db);

    assert_eq!(
        repository
            .ensure_project_slug("Ppath")
            .expect("ensure project slug")
            .slug,
        "path-fallback-app"
    );
    let fallback = repository
        .ensure_project_slug("Pempty")
        .expect("ensure project fallback slug")
        .slug;
    assert!(fallback.starts_with("project-"));
    assert_eq!(
        repository
            .ensure_project_slug("Pempty")
            .expect("ensure project fallback slug again")
            .slug,
        fallback
    );
}

#[test]
fn project_slug_collisions_keep_first_clean_slug_and_stable_later_suffixes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pfirst",
        "Ghostex",
        Some("/tmp/first"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_project_with_path(
        &db,
        "Psecond",
        "Ghostex",
        Some("/tmp/second"),
        "2026-06-22T18:42:00.000Z",
    );
    let repository = PortlessRepository::new(&db);

    let first_backfill = repository
        .backfill_domain_identities()
        .expect("first backfill");
    let first_slug = project_slug(&first_backfill, "Pfirst");
    let second_slug = project_slug(&first_backfill, "Psecond");
    assert_eq!(first_slug, "ghostex");
    assert!(second_slug.starts_with("ghostex-"));
    assert_ne!(first_slug, second_slug);

    db.execute(
        "UPDATE projects SET name = ?2, updatedAt = ?3 WHERE projectId = ?1",
        params!["Pfirst", "Renamed Away", "2026-06-22T18:45:00.000Z"],
    )
    .expect("rename first project");
    let second_backfill = repository
        .backfill_domain_identities()
        .expect("second backfill");
    assert_eq!(project_slug(&second_backfill, "Pfirst"), "ghostex");
    assert_eq!(project_slug(&second_backfill, "Psecond"), second_slug);
    assert_eq!(
        sorted_project_slug_pairs(&first_backfill),
        sorted_project_slug_pairs(&second_backfill)
    );
}

#[test]
fn worktree_suffix_persists_across_worktree_name_and_branch_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pparent",
        "Ghostex",
        Some("/tmp/ghostex"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtfix",
        "Pparent",
        "Worktree Display",
        "Fix UI",
        "feature/fix-ui",
        "2026-06-22T18:42:00.000Z",
    );
    let repository = PortlessRepository::new(&db);

    let created = repository
        .ensure_worktree_slug("Pwtfix")
        .expect("ensure worktree slug");
    assert_eq!(created.parent_project_id, "Pparent");
    assert_eq!(created.project_slug, "ghostex");
    assert_eq!(created.worktree_key, "Pwtfix");
    assert_eq!(created.worktree_slug, "fix-ui");

    update_worktree_metadata(
        &db,
        "Pwtfix",
        "Pparent",
        "Renamed Worktree",
        "feature/renamed-worktree",
    );
    let after_rename = repository
        .ensure_worktree_slug("Pwtfix")
        .expect("ensure after rename");
    assert_eq!(after_rename.worktree_slug, "fix-ui");
    assert_eq!(after_rename.project_slug, "ghostex");
}

#[test]
fn worktree_suffix_uses_name_first_then_branch_last_segment() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pparent",
        "Ghostex",
        Some("/tmp/ghostex"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtname",
        "Pparent",
        "Name Project",
        "Release Prep",
        "feature/ignored-branch",
        "2026-06-22T18:42:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtbranch",
        "Pparent",
        "Branch Project",
        "!!!",
        "refs/heads/feature/fix/login",
        "2026-06-22T18:43:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtfallback",
        "Pparent",
        "Fallback Project",
        "!!!",
        "///",
        "2026-06-22T18:44:00.000Z",
    );
    let repository = PortlessRepository::new(&db);

    let identities = repository
        .backfill_domain_identities()
        .expect("backfill identities");
    assert_eq!(worktree_slug(&identities, "Pwtname"), "release-prep");
    assert_eq!(worktree_slug(&identities, "Pwtbranch"), "login");
    assert!(worktree_slug(&identities, "Pwtfallback").starts_with("wt-"));
}

#[test]
fn worktree_suffix_collisions_are_stable_per_parent_without_reshuffling() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = get_gxserver_paths(Some(temp.path().to_path_buf()));
    initialize_gxserver_storage(&paths).expect("storage init");
    let db = open_gxserver_database(&paths).expect("open db");
    insert_project_with_path(
        &db,
        "Pparent",
        "Ghostex",
        Some("/tmp/ghostex"),
        "2026-06-22T18:41:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtfirst",
        "Pparent",
        "First Worktree",
        "Fix UI",
        "feature/fix-ui-a",
        "2026-06-22T18:42:00.000Z",
    );
    insert_worktree_project(
        &db,
        "Pwtsecond",
        "Pparent",
        "Second Worktree",
        "Fix UI",
        "feature/fix-ui-b",
        "2026-06-22T18:43:00.000Z",
    );
    let repository = PortlessRepository::new(&db);

    let first_backfill = repository
        .backfill_domain_identities()
        .expect("first backfill");
    let first_suffix = worktree_slug(&first_backfill, "Pwtfirst");
    let second_suffix = worktree_slug(&first_backfill, "Pwtsecond");
    assert_eq!(first_suffix, "fix-ui");
    assert!(second_suffix.starts_with("fix-ui-"));
    assert_ne!(first_suffix, second_suffix);

    update_worktree_metadata(&db, "Pwtfirst", "Pparent", "Other Name", "feature/other");
    let second_backfill = repository
        .backfill_domain_identities()
        .expect("second backfill");
    assert_eq!(worktree_slug(&second_backfill, "Pwtfirst"), "fix-ui");
    assert_eq!(worktree_slug(&second_backfill, "Pwtsecond"), second_suffix);
    assert_eq!(
        sorted_worktree_slug_pairs(&first_backfill),
        sorted_worktree_slug_pairs(&second_backfill)
    );
}
