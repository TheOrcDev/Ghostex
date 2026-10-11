//! `cargo xtask remote-start`: build the Linux x64 desktop app on a rented Blacksmith machine, then install and launch it here. `cargo xtask remote-build-pack` is the half that runs on that machine.
//!
//! CDXC:Build 2026-10-02 DECISION:
//! User chose Blacksmith's per-minute machines to build the Linux x64 desktop app because their Linux laptop is too weak to build it.
//! The snapshot goes to a private repository in a GitHub organization (Blacksmith only serves organization repositories), never to the public one, so work in progress stays private.
//!
//! CDXC:Build 2026-10-02 WHY:
//! The staged app is about 1.8 GB, almost all of it CEF, which changes only with a CEF upgrade. The laptop sends a checksum list of its own staged app inside the snapshot and the build machine returns only the files that differ, so an ordinary rebuild downloads the two binaries instead of the whole app.
//! SEE-ALSO: tooling/remote-build/linux-x64.yml (the workflow), `cargo xtask start --install-only` (tooling/xtask/src/start/mod.rs).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::bail;
use crate::start;
use crate::util::{self, env_trimmed, format_duration, output, root, sleep_ms, Res};

const BRANCH: &str = "remote-build/linux-x64";
/// The build repository's default branch, which holds only the workflow so workflow_dispatch can find it.
const DISPATCH_BRANCH: &str = "main";
const WORKFLOW_TEMPLATE: &str = "tooling/remote-build/linux-x64.yml";
const WORKFLOW_FILE: &str = "remote-build-linux-x64.yml";
const ARTIFACT: &str = "remote-build-linux-x64";
const LOCAL_REF: &str = "refs/remote-build/linux-x64";
const REPO_CONFIG_KEY: &str = "ghostex.remoteBuildRepo";
const STAGED_APP: &str = "apps/desktop/build/linux/Ghostex";
/// The submodules the workflow checks out; a commit in one of them must be on GitHub before it can be built remotely.
const BUILD_SUBMODULES: [&str; 5] = [
    ".dependencies/zmx",
    ".dependencies/zed",
    ".dependencies/cef-rs",
    ".dependencies/gpui-component",
    ".dependencies/code-server",
];
/// GitHub refuses any pushed file over 100 MB.
const MAX_FILE_BYTES: u64 = 95 * 1024 * 1024;

const USAGE: &str = "cargo xtask remote-start [--repo <owner/name>] [--build-only]";

pub fn run(args: &[String]) -> Res<i32> {
    let mut repo_arg = None;
    let mut build_only = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--" => {}
            "--build-only" => build_only = true,
            "--repo" => {
                repo_arg = Some(
                    iter.next()
                        .cloned()
                        .ok_or_else(|| format!("--repo needs <owner/name>. Usage: {USAGE}"))?,
                )
            }
            other if other.starts_with("--repo=") => {
                repo_arg = Some(other["--repo=".len()..].to_string())
            }
            other => bail!("Unknown remote-start argument: {other}. Usage: {USAGE}"),
        }
    }
    if !(cfg!(target_os = "linux") && cfg!(target_arch = "x86_64")) || is_wsl() {
        bail!("Remote builds cover the Linux x64 desktop app for now; run this on a Linux x64 computer.");
    }
    let repo = resolve_repo(repo_arg)?;
    let started = Instant::now();
    let work = root().join("build").join("remote-build").join("linux-x64");
    fs::create_dir_all(&work)?;
    let stage = root().join(STAGED_APP);

    println!("[1] Taking a snapshot of the working tree...");
    let mut cache = HashCache::load(work.join("hash-cache.tsv"));
    let base = manifest_of(&stage, Some(&mut cache))?;
    cache.save()?;
    if base.is_empty() {
        println!("    There is no staged build here yet, so this first download is the whole app (several hundred MB).");
    }
    let snapshot = snapshot(&work, &manifest_text(&base))?;
    println!(
        "    Snapshot {} of {}{}.",
        short(&snapshot.commit),
        short(&snapshot.head),
        if snapshot.local_changes {
            " with your uncommitted changes"
        } else {
            ""
        }
    );
    for note in &snapshot.notes {
        println!("    Note: {note}");
    }

    println!("[2] Pushing it to {repo}...");
    ensure_dispatch_workflow(&repo)?;
    push(&repo, &snapshot.commit, BRANCH)?;

    println!("[3] Building on Blacksmith...");
    dispatch(&repo)?;
    let run_id = find_run(&repo, &snapshot.commit)?;
    println!("    https://github.com/{repo}/actions/runs/{run_id}");
    println!("    (Ctrl+C stops watching here; the build keeps running and can be cancelled on that page.)");
    watch_run(&repo, run_id)?;

    println!("[4] Downloading the files that changed...");
    let download = work.join("download");
    download_artifact(&repo, run_id, &download)?;
    let summary = {
        let _lock = util::acquire_start_lock("remote-start")?;
        let summary = apply(&stage, &download, &base, &mut cache)?;
        cache.save()?;
        summary
    };
    println!("    {summary}");
    println!(
        "    Remote build done in {}.",
        format_duration(started.elapsed())
    );
    if build_only {
        println!(
            "Staged {}; nothing was installed or launched.",
            stage.display()
        );
        return Ok(0);
    }
    start::run(&["--install-only".to_string()])
}

/// `cargo xtask remote-build-pack --stage <dir> --base <manifest> --out <dir>`, run by the workflow after the build.
pub fn pack(args: &[String]) -> Res<i32> {
    let mut stage = None;
    let mut base = None;
    let mut out = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let slot = match arg.as_str() {
            "--stage" => &mut stage,
            "--base" => &mut base,
            "--out" => &mut out,
            other => bail!("Unknown remote-build-pack argument: {other}."),
        };
        *slot = Some(
            iter.next()
                .map(|value| std::env::current_dir().unwrap_or_default().join(value))
                .ok_or_else(|| format!("{arg} needs a path."))?,
        );
    }
    let (Some(stage), Some(base), Some(out)) = (stage, base, out) else {
        bail!("Usage: cargo xtask remote-build-pack --stage <dir> --base <manifest> --out <dir>");
    };
    let current = manifest_of(&stage, None)?;
    if current.is_empty() {
        bail!("{} holds no build.", stage.display());
    }
    let base = match fs::read_to_string(&base) {
        Ok(text) => parse_manifest(&text)?,
        Err(_) => Manifest::new(),
    };
    let changed: Vec<&String> = current
        .iter()
        .filter(|(path, entry)| base.get(*path) != Some(*entry))
        .map(|(path, _)| path)
        .collect();
    fs::create_dir_all(&out)?;
    fs::write(out.join("manifest.tsv"), manifest_text(&current))?;
    let list = out.join("changed.txt");
    fs::write(
        &list,
        changed
            .iter()
            .map(|path| format!("{path}\n"))
            .collect::<String>(),
    )?;
    let archive = out.join("changed.tar.zst");
    util::check(
        Command::new("tar")
            .arg("-C")
            .arg(&stage)
            .args(["-I", "zstd -T0 -6", "-cf"])
            .arg(&archive)
            .args(["--verbatim-files-from", "-T"])
            .arg(&list),
    )?;
    let changed_bytes: u64 = changed.iter().map(|path| current[*path].size).sum();
    let total_bytes: u64 = current.values().map(|entry| entry.size).sum();
    println!(
        "{} of {} files differ from the laptop's staged app ({} of {}); archive {}.",
        changed.len(),
        current.len(),
        megabytes(changed_bytes),
        megabytes(total_bytes),
        megabytes(fs::metadata(&archive)?.len())
    );
    Ok(0)
}

fn is_wsl() -> bool {
    env_trimmed("WSL_DISTRO_NAME").is_some()
        || fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_default()
            .to_lowercase()
            .contains("microsoft")
}

/// The build repository comes from `--repo` (saved in this checkout's git config), GHOSTEX_REMOTE_BUILD_REPO, or that saved value, and must be a private organization repository.
fn resolve_repo(arg: Option<String>) -> Res<String> {
    let from_arg = arg.is_some();
    let repo = arg
        .or_else(|| env_trimmed("GHOSTEX_REMOTE_BUILD_REPO"))
        .or_else(|| {
            util::stdout_if_ok(util::command("git").args(["config", "--get", REPO_CONFIG_KEY]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
    let Some(repo) = repo else {
        bail!("No build repository is set. Create a private repository in a GitHub organization that has the Blacksmith app installed, then run `cargo xtask remote-start --repo <org>/<name>` once (see tooling/remote-build/README.md).");
    };
    let valid = repo.split('/').count() == 2
        && repo.split('/').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        });
    if !valid {
        bail!("The build repository must look like <owner>/<name>, not {repo}.");
    }
    let details = output(util::command("gh").args([
        "api",
        &format!("repos/{repo}"),
        "--jq",
        "[.private, .owner.type] | @tsv",
    ]))
    .map_err(|_| "The GitHub CLI (`gh`) is required; install it and run `gh auth login`.")?;
    if !details.status.success() {
        bail!(
            "Cannot reach {repo} with the GitHub CLI: {}",
            String::from_utf8_lossy(&details.stderr).trim()
        );
    }
    let details = String::from_utf8_lossy(&details.stdout);
    let mut fields = details.trim().split('\t');
    if fields.next() != Some("true") {
        bail!("{repo} is public. Snapshots include your uncommitted work, so the build repository must be private.");
    }
    if fields.next() != Some("Organization") {
        bail!("{repo} belongs to a personal account. Blacksmith only runs jobs for repositories owned by a GitHub organization.");
    }
    if from_arg {
        util::check(util::command("git").args(["config", "--local", REPO_CONFIG_KEY, &repo]))?;
        println!(
            "Saved {repo} as this checkout's build repository (git config {REPO_CONFIG_KEY})."
        );
    }
    Ok(repo)
}

struct Snapshot {
    commit: String,
    head: String,
    local_changes: bool,
    notes: Vec<String>,
}

/// CDXC:Build 2026-10-02 WHY:
/// Other agents keep uncommitted work in this checkout, so the snapshot is built in a private copy of the index and never touches the real index, the working tree, or the branch.
/// It is a parentless commit so the first push sends one tree instead of the whole history; later pushes send only changed files because the last snapshot stays referenced locally (LOCAL_REF) and on the build repository.
/// The checkout's own workflows are left out so pushing to the build repository can only start the remote build workflow.
fn snapshot(work: &Path, base_manifest: &str) -> Res<Snapshot> {
    let index = work.join("snapshot-index");
    let real_index = PathBuf::from(git_text(git(None).args([
        "rev-parse",
        "--path-format=absolute",
        "--git-path",
        "index",
    ]))?);
    let _ = fs::remove_file(&index);
    if real_index.exists() {
        fs::copy(&real_index, &index)?;
    }
    git_text(git(Some(&index)).args(["add", "--all"]))?;
    let head = git_text(git(None).args(["rev-parse", "HEAD"]))?;
    let changed = git_text(git(Some(&index)).args([
        "diff",
        "--cached",
        "--name-only",
        "--no-renames",
        "-z",
        "HEAD",
    ]))?;
    let changed: Vec<&str> = changed.split('\0').filter(|p| !p.is_empty()).collect();
    for path in &changed {
        if let Ok(meta) = fs::metadata(root().join(path)) {
            if meta.is_file() && meta.len() > MAX_FILE_BYTES {
                bail!(
                    "{path} is {} and GitHub refuses files over 100 MB. Delete it or add it to .gitignore, then run again.",
                    megabytes(meta.len())
                );
            }
        }
    }
    git_text(git(Some(&index)).args([
        "rm",
        "-r",
        "-f",
        "--cached",
        "--quiet",
        "--ignore-unmatch",
        "--",
        ".github/workflows",
    ]))?;
    let request = format!(
        "GX_REMOTE_ROOT={}\nGX_REMOTE_SOURCE_HEAD={head}\n",
        root().display()
    );
    for (path, content) in [
        (
            format!(".github/workflows/{WORKFLOW_FILE}"),
            fs::read(root().join(WORKFLOW_TEMPLATE))?,
        ),
        (
            ".remote-build/base-manifest.tsv".to_string(),
            base_manifest.as_bytes().to_vec(),
        ),
        (
            ".remote-build/request.env".to_string(),
            request.into_bytes(),
        ),
    ] {
        let blob = git_text_with_input(git(None).args(["hash-object", "-w", "--stdin"]), &content)?;
        git_text(git(Some(&index)).args([
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("100644,{blob},{path}"),
        ]))?;
    }
    let tree = git_text(git(Some(&index)).arg("write-tree"))?;
    let notes = check_submodules(&tree)?;
    let commit = git_text(git(None).args([
        "commit-tree",
        &tree,
        "-m",
        &format!("Remote build snapshot of {head}"),
    ]))?;
    git_text(git(None).args(["update-ref", LOCAL_REF, &commit]))?;
    let _ = fs::remove_file(&index);
    Ok(Snapshot {
        commit,
        head,
        local_changes: !changed.is_empty(),
        notes,
    })
}

/// A submodule checked out at a commit GitHub does not have would fail on the build machine, so stop here; edits inside a submodule cannot travel in the snapshot, so say so.
fn check_submodules(tree: &str) -> Res<Vec<String>> {
    let pinned = gitlinks("HEAD")?;
    let snapshot = gitlinks(tree)?;
    let mut notes = Vec::new();
    for path in BUILD_SUBMODULES {
        let Some(revision) = snapshot.get(path) else {
            continue;
        };
        let checkout = root().join(path);
        if !checkout.join(".git").exists() {
            continue;
        }
        if pinned.get(path) != Some(revision) {
            let on_remote =
                util::stdout_if_ok(util::command("git").arg("-C").arg(&checkout).args([
                    "branch",
                    "-r",
                    "--contains",
                    revision,
                ]))
                .unwrap_or_default();
            if on_remote.trim().is_empty() {
                bail!(
                    "{path} is checked out at {}, which is on no remote branch. The build machine downloads submodules from GitHub, so push that commit first.",
                    short(revision)
                );
            }
            notes.push(format!(
                "{path} builds at your checked-out {} instead of the pinned commit.",
                short(revision)
            ));
        }
        let dirty = util::stdout_if_ok(util::command("git").arg("-C").arg(&checkout).args([
            "status",
            "--porcelain",
            "--untracked-files=no",
        ]))
        .unwrap_or_default();
        if !dirty.trim().is_empty() {
            notes.push(format!(
                "uncommitted edits inside {path} are not included; commit and push them in that submodule to build them remotely."
            ));
        }
    }
    Ok(notes)
}

fn gitlinks(treeish: &str) -> Res<BTreeMap<String, String>> {
    let listing = git_text(git(None).args(["ls-tree", "-r", "-z", treeish]))?;
    let mut links = BTreeMap::new();
    for record in listing.split('\0') {
        let Some((meta, path)) = record.split_once('\t') else {
            continue;
        };
        let mut fields = meta.split(' ');
        if fields.next() == Some("160000") {
            if let Some(revision) = fields.nth(1) {
                links.insert(path.to_string(), revision.to_string());
            }
        }
    }
    Ok(links)
}

/// CDXC:Build 2026-10-02 WHY:
/// GitHub started no workflow at all for the snapshot pushes (a parentless commit adding about 10,600 files), while a tiny probe push on the same repository ran at once, so the build is started with workflow_dispatch instead of relying on the push event.
/// workflow_dispatch only finds workflows on the default branch, so the build repository's `main` holds just this workflow file; the run itself uses the copy inside the snapshot.
fn ensure_dispatch_workflow(repo: &str) -> Res {
    let template = fs::read(root().join(WORKFLOW_TEMPLATE))?;
    let blob = git_text_with_input(git(None).args(["hash-object", "-w", "--stdin"]), &template)?;
    let published = util::stdout_if_ok(util::command("gh").args([
        "api",
        &format!("repos/{repo}/contents/.github/workflows/{WORKFLOW_FILE}?ref={DISPATCH_BRANCH}"),
        "--jq",
        ".sha",
    ]));
    let default_branch = util::stdout_if_ok(util::command("gh").args([
        "api",
        &format!("repos/{repo}"),
        "--jq",
        ".default_branch",
    ]))
    .unwrap_or_default();
    if published.as_deref().map(str::trim) == Some(blob.as_str())
        && default_branch.trim() == DISPATCH_BRANCH
    {
        return Ok(());
    }
    let mktree = |entry: String| {
        git_text_with_input(git(None).arg("mktree"), format!("{entry}\n").as_bytes())
    };
    let workflows = mktree(format!("100644 blob {blob}\t{WORKFLOW_FILE}"))?;
    let github = mktree(format!("040000 tree {workflows}\tworkflows"))?;
    let tree = mktree(format!("040000 tree {github}\t.github"))?;
    let commit = git_text(git(None).args(["commit-tree", &tree, "-m", "Remote build workflow"]))?;
    push(repo, &commit, DISPATCH_BRANCH)?;
    if default_branch.trim() != DISPATCH_BRANCH {
        util::check(util::command("gh").args([
            "api",
            "--silent",
            "-X",
            "PATCH",
            &format!("repos/{repo}"),
            "-f",
            &format!("default_branch={DISPATCH_BRANCH}"),
        ]))?;
    }
    Ok(())
}

/// Starts the workflow on the snapshot branch, retrying while GitHub registers a just-published workflow.
fn dispatch(repo: &str) -> Res {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let out = output(util::command("gh").args([
            "workflow",
            "run",
            WORKFLOW_FILE,
            "-R",
            repo,
            "--ref",
            BRANCH,
        ]))?;
        if out.status.success() {
            return Ok(());
        }
        if Instant::now() > deadline {
            bail!(
                "GitHub refused to start the build workflow: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        sleep_ms(5000);
    }
}

fn push(repo: &str, commit: &str, branch: &str) -> Res {
    util::check(util::command("git").args([
        "-c",
        "credential.helper=",
        "-c",
        "credential.helper=!gh auth git-credential",
        "push",
        "--force",
        "--no-verify",
        &format!("https://github.com/{repo}.git"),
        &format!("{commit}:refs/heads/{branch}"),
    ]))
}

fn find_run(repo: &str, commit: &str) -> Res<u64> {
    let deadline = Instant::now() + Duration::from_secs(120);
    let select = format!(".[] | select(.headSha == \"{commit}\") | .databaseId");
    loop {
        let found = util::stdout_if_ok(util::command("gh").args([
            "run",
            "list",
            "-R",
            repo,
            "--workflow",
            WORKFLOW_FILE,
            "--branch",
            BRANCH,
            "--limit",
            "20",
            "--json",
            "databaseId,headSha",
            "--jq",
            &select,
        ]));
        if let Some(id) = found
            .as_deref()
            .and_then(|text| text.lines().next())
            .and_then(|line| line.trim().parse().ok())
        {
            return Ok(id);
        }
        if Instant::now() > deadline {
            bail!("GitHub did not start the build workflow within 2 minutes. Check that Actions are enabled for the repository: https://github.com/{repo}/actions");
        }
        sleep_ms(3000);
    }
}

fn watch_run(repo: &str, run_id: u64) -> Res {
    let started = Instant::now();
    let id = run_id.to_string();
    let progress = r#"[.status, (.conclusion // ""), ([.jobs[]?.steps[]? | select(.status == "in_progress") | .name] | first // "")] | @tsv"#;
    let mut last = String::new();
    let mut misses = 0;
    let mut warned = false;
    loop {
        match util::stdout_if_ok(util::command("gh").args([
            "run",
            "view",
            &id,
            "-R",
            repo,
            "--json",
            "status,conclusion,jobs",
            "--jq",
            progress,
        ])) {
            None => {
                misses += 1;
                if misses >= 30 {
                    bail!("Lost contact with GitHub while watching https://github.com/{repo}/actions/runs/{id}");
                }
            }
            Some(line) => {
                misses = 0;
                let mut fields = line.trim_end_matches('\n').split('\t');
                let status = fields.next().unwrap_or_default();
                let conclusion = fields.next().unwrap_or_default();
                let step = fields.next().unwrap_or_default();
                if status == "completed" {
                    if conclusion == "success" {
                        println!("    [{}] Build finished.", clock(started.elapsed()));
                        record_step_timings(repo, &id);
                        return Ok(());
                    }
                    record_step_timings(repo, &id);
                    print_failed_log(repo, &id);
                    bail!("The remote build ended with '{conclusion}': https://github.com/{repo}/actions/runs/{id}");
                }
                let label = match (status, step) {
                    (_, step) if !step.is_empty() => step,
                    ("in_progress", _) => "Starting",
                    _ => "Waiting for a Blacksmith machine",
                };
                if label != last {
                    println!("    [{}] {label}", clock(started.elapsed()));
                    last = label.to_string();
                }
                if status != "in_progress"
                    && !warned
                    && started.elapsed() > Duration::from_secs(180)
                {
                    println!("    Still waiting for a machine after 3 minutes. Blacksmith only takes jobs from organization repositories that have its GitHub app installed (https://app.blacksmith.sh).");
                    warned = true;
                }
            }
        }
        sleep_ms(10_000);
    }
}

/// CDXC:Build 2026-10-02 DECISION:
/// User asked to track how long each part of a remote build takes, to decide whether to do all dev builds on Blacksmith. Every finished run prints its step durations and appends them to build/remote-build/linux-x64/timings.tsv (run, date, step, seconds).
fn record_step_timings(repo: &str, id: &str) {
    let steps = r#".jobs[0] as $job | ($job.steps[]? | select(.status == "completed" and .startedAt != null) | [.name, ((.completedAt | fromdateiso8601) - (.startedAt | fromdateiso8601))] | @tsv), (["Whole job", (($job.completedAt | fromdateiso8601) - ($job.startedAt | fromdateiso8601))] | @tsv)"#;
    let Some(table) = util::stdout_if_ok(util::command("gh").args([
        "run", "view", id, "-R", repo, "--json", "jobs", "--jq", steps,
    ])) else {
        return;
    };
    let date = util::local_timestamp();
    let mut log = String::new();
    println!("    Step timings:");
    for line in table.lines() {
        let Some((name, seconds)) = line.split_once('\t') else {
            continue;
        };
        let seconds: u64 = seconds.trim().parse().unwrap_or(0);
        if seconds >= 1 || name == "Whole job" {
            println!("      {:>7}  {name}", clock(Duration::from_secs(seconds)));
        }
        log.push_str(&format!("{id}\t{date}\t{name}\t{seconds}\n"));
    }
    let path = root().join("build/remote-build/linux-x64/timings.tsv");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(log.as_bytes());
        println!("      (all runs: {})", path.display());
    }
}

fn print_failed_log(repo: &str, id: &str) {
    let Some(log) = util::stdout_if_ok(util::command("gh").args([
        "run",
        "view",
        id,
        "-R",
        repo,
        "--log-failed",
    ])) else {
        return;
    };
    let lines: Vec<&str> = log.lines().collect();
    println!("    Last lines of the failed step:");
    for line in &lines[lines.len().saturating_sub(80)..] {
        println!("    | {line}");
    }
}

/// Downloads the build's changed files, then deletes the artifact so it does not count against the organization's storage.
fn download_artifact(repo: &str, run_id: u64, destination: &Path) -> Res {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    util::check(
        util::command("gh")
            .args([
                "run",
                "download",
                &run_id.to_string(),
                "-R",
                repo,
                "-n",
                ARTIFACT,
                "-D",
            ])
            .arg(destination),
    )?;
    if let Some(ids) = util::stdout_if_ok(util::command("gh").args([
        "api",
        &format!("repos/{repo}/actions/runs/{run_id}/artifacts"),
        "--jq",
        ".artifacts[].id",
    ])) {
        for artifact in ids.lines().map(str::trim).filter(|id| !id.is_empty()) {
            let _ = util::stdout_if_ok(util::command("gh").args([
                "api",
                "-X",
                "DELETE",
                &format!("repos/{repo}/actions/artifacts/{artifact}"),
            ]));
        }
    }
    Ok(())
}

/// Puts the downloaded files into the staged app after checking every checksum, and removes files the new build no longer has.
fn apply(stage: &Path, download: &Path, base: &Manifest, cache: &mut HashCache) -> Res<String> {
    let target = parse_manifest(&fs::read_to_string(download.join("manifest.tsv"))?)?;
    let changed: Vec<String> = fs::read_to_string(download.join("changed.txt"))?
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    let changed_set: BTreeSet<&str> = changed.iter().map(String::as_str).collect();
    let archive = download.join("changed.tar.zst");
    let archive_bytes = fs::metadata(&archive)?.len();
    let incoming = download.join("incoming");
    fs::create_dir_all(&incoming)?;
    util::check(
        Command::new("tar")
            .arg("-C")
            .arg(&incoming)
            .args(["-I", "zstd", "-xf"])
            .arg(&archive),
    )?;

    let changed_files: Vec<&str> = changed
        .iter()
        .filter(|path| {
            target
                .get(*path)
                .is_some_and(|entry| entry.kind == Kind::File)
        })
        .map(String::as_str)
        .collect();
    let hashes = sha256_files(&incoming, &changed_files)?;
    for path in &changed {
        let entry = target
            .get(path)
            .ok_or_else(|| format!("{path} was sent but is not in the build's file list."))?;
        let intact = match entry.kind {
            Kind::File => hashes.get(path) == Some(&entry.digest),
            Kind::Link => fs::read_link(incoming.join(path))
                .is_ok_and(|link| link.to_str() == Some(entry.digest.as_str())),
        };
        if !intact {
            bail!("{path} arrived damaged (it does not match the build's checksum). Run `cargo xtask remote-start` again.");
        }
    }
    let current = manifest_of(stage, Some(cache))?;
    if current != *base {
        bail!(
            "{} changed while the remote build ran (a local build?), so the downloaded files no longer fit it. Run `cargo xtask remote-start` again.",
            stage.display()
        );
    }
    if let Some((path, _)) = target.iter().find(|(path, entry)| {
        !changed_set.contains(path.as_str()) && current.get(*path) != Some(*entry)
    }) {
        bail!("The build's file list expects {path} to be unchanged, but it differs here. Run `cargo xtask remote-start` again.");
    }

    for path in &changed {
        let destination = stage.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_entry(&destination)?;
        move_entry(&incoming.join(path), &destination)?;
        let entry = &target[path];
        if entry.kind == Kind::File {
            set_mode(&destination, entry.mode)?;
            let meta = fs::metadata(&destination)?;
            cache.entries.insert(
                path.clone(),
                (meta.len(), mtime_ns(&meta), entry.digest.clone()),
            );
        }
    }
    let mut removed = 0;
    for path in current.keys().filter(|path| !target.contains_key(*path)) {
        remove_entry(&stage.join(path))?;
        cache.entries.remove(path);
        removed += 1;
    }
    let _ = fs::remove_dir_all(&incoming);
    Ok(format!(
        "{} file{} updated, {removed} removed ({} downloaded).",
        changed.len(),
        if changed.len() == 1 { "" } else { "s" },
        megabytes(archive_bytes)
    ))
}

fn remove_entry(path: &Path) -> Res {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(_) => {}
    }
    Ok(())
}

/// A rename, or a copy when the download folder sits on another filesystem.
fn move_entry(from: &Path, to: &Path) -> Res {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        make_symlink(&fs::read_link(from)?, to)?;
    } else {
        fs::copy(from, to)?;
    }
    fs::remove_file(from)?;
    Ok(())
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> Res {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_symlink(_target: &Path, link: &Path) -> Res {
    bail!(
        "Cannot create the symlink {} on this platform.",
        link.display()
    )
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Res {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Res {
    Ok(())
}

#[cfg(unix)]
fn file_mode(meta: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o7777
}

#[cfg(not(unix))]
fn file_mode(_meta: &fs::Metadata) -> u32 {
    0o644
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    File,
    Link,
}

/// One staged file: its checksum (or a symlink's target), size and permission bits.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Entry {
    kind: Kind,
    mode: u32,
    size: u64,
    digest: String,
}

type Manifest = BTreeMap<String, Entry>;

/// `kind<TAB>mode<TAB>size<TAB>sha256-or-link-target<TAB>path`, one line per file, sorted by path.
fn manifest_text(manifest: &Manifest) -> String {
    manifest
        .iter()
        .map(|(path, entry)| {
            format!(
                "{}\t{:o}\t{}\t{}\t{path}\n",
                if entry.kind == Kind::File { 'f' } else { 'l' },
                entry.mode,
                entry.size,
                entry.digest
            )
        })
        .collect()
}

fn parse_manifest(text: &str) -> Res<Manifest> {
    let mut manifest = Manifest::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.splitn(5, '\t').collect();
        let [kind, mode, size, digest, path] = fields[..] else {
            bail!("Unreadable file list line: {line}");
        };
        manifest.insert(
            path.to_string(),
            Entry {
                kind: if kind == "l" { Kind::Link } else { Kind::File },
                mode: u32::from_str_radix(mode, 8)?,
                size: size.parse()?,
                digest: digest.to_string(),
            },
        );
    }
    Ok(manifest)
}

/// Checksums every file under `stage`; with a cache, files whose size and modification time are unchanged reuse their last checksum.
fn manifest_of(stage: &Path, mut cache: Option<&mut HashCache>) -> Res<Manifest> {
    let mut manifest = Manifest::new();
    let mut pending = Vec::new();
    for found in walk(stage)? {
        if found.kind == Kind::Link {
            manifest.insert(
                found.path,
                Entry {
                    kind: Kind::Link,
                    mode: 0,
                    size: 0,
                    digest: found.link,
                },
            );
            continue;
        }
        let cached = cache
            .as_deref()
            .and_then(|cache| cache.entries.get(&found.path))
            .filter(|(size, mtime, _)| *size == found.size && *mtime == found.mtime)
            .map(|(_, _, digest)| digest.clone());
        match cached {
            Some(digest) => {
                manifest.insert(
                    found.path.clone(),
                    Entry {
                        kind: Kind::File,
                        mode: found.mode,
                        size: found.size,
                        digest,
                    },
                );
            }
            None => pending.push(found),
        }
    }
    let paths: Vec<&str> = pending.iter().map(|found| found.path.as_str()).collect();
    let hashes = sha256_files(stage, &paths)?;
    for found in pending {
        let digest = hashes
            .get(&found.path)
            .cloned()
            .ok_or_else(|| format!("sha256sum skipped {}", found.path))?;
        if let Some(cache) = cache.as_deref_mut() {
            cache.entries.insert(
                found.path.clone(),
                (found.size, found.mtime, digest.clone()),
            );
        }
        manifest.insert(
            found.path,
            Entry {
                kind: Kind::File,
                mode: found.mode,
                size: found.size,
                digest,
            },
        );
    }
    if let Some(cache) = cache {
        cache.entries.retain(|path, _| manifest.contains_key(path));
    }
    Ok(manifest)
}

struct Found {
    path: String,
    kind: Kind,
    mode: u32,
    size: u64,
    mtime: i128,
    link: String,
}

fn walk(stage: &Path) -> Res<Vec<Found>> {
    let mut found = Vec::new();
    if !stage.is_dir() {
        return Ok(found);
    }
    let mut folders = vec![PathBuf::new()];
    while let Some(folder) = folders.pop() {
        for item in fs::read_dir(stage.join(&folder))? {
            let item = item?;
            let relative = folder.join(item.file_name());
            let path = relative
                .to_str()
                .filter(|path| !path.contains(['\t', '\n', '\\']))
                .ok_or_else(|| {
                    format!(
                        "Unsupported file name in the staged app: {}",
                        relative.display()
                    )
                })?
                .to_string();
            let meta = fs::symlink_metadata(item.path())?;
            let kind = meta.file_type();
            if kind.is_dir() {
                folders.push(relative);
            } else if kind.is_symlink() {
                let link = fs::read_link(item.path())?
                    .to_str()
                    .ok_or_else(|| format!("Unsupported symlink target in {path}"))?
                    .to_string();
                found.push(Found {
                    path,
                    kind: Kind::Link,
                    mode: 0,
                    size: 0,
                    mtime: 0,
                    link,
                });
            } else if kind.is_file() {
                found.push(Found {
                    path,
                    kind: Kind::File,
                    mode: file_mode(&meta),
                    size: meta.len(),
                    mtime: mtime_ns(&meta),
                    link: String::new(),
                });
            }
        }
    }
    Ok(found)
}

fn mtime_ns(meta: &fs::Metadata) -> i128 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos() as i128)
}

/// SHA-256 of files relative to `folder`, with coreutils' sha256sum (present on every Linux this runs on).
fn sha256_files(folder: &Path, paths: &[&str]) -> Res<HashMap<String, String>> {
    let mut hashes = HashMap::new();
    for chunk in paths.chunks(256) {
        let out = output(
            Command::new("sha256sum")
                .arg("--")
                .args(chunk)
                .current_dir(folder),
        )?;
        if !out.status.success() {
            bail!(
                "sha256sum failed in {}: {}",
                folder.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let Some((digest, path)) = line.split_once("  ") else {
                bail!("Unexpected sha256sum output: {line}");
            };
            hashes.insert(path.to_string(), digest.to_string());
        }
    }
    Ok(hashes)
}

/// Remembered checksums of the staged app, keyed by path, size and modification time, so a snapshot does not re-read 1.8 GB each time.
struct HashCache {
    path: PathBuf,
    entries: HashMap<String, (u64, i128, String)>,
}

impl HashCache {
    fn load(path: PathBuf) -> Self {
        let mut entries = HashMap::new();
        for line in fs::read_to_string(&path).unwrap_or_default().lines() {
            let fields: Vec<&str> = line.splitn(4, '\t').collect();
            if let [size, mtime, digest, file] = fields[..] {
                if let (Ok(size), Ok(mtime)) = (size.parse(), mtime.parse()) {
                    entries.insert(file.to_string(), (size, mtime, digest.to_string()));
                }
            }
        }
        Self { path, entries }
    }

    fn save(&self) -> Res {
        let mut lines: Vec<String> = self
            .entries
            .iter()
            .map(|(file, (size, mtime, digest))| format!("{size}\t{mtime}\t{digest}\t{file}\n"))
            .collect();
        lines.sort();
        let partial = self.path.with_extension("partial");
        fs::write(&partial, lines.concat())?;
        fs::rename(&partial, &self.path)?;
        Ok(())
    }
}

fn git(index: Option<&Path>) -> Command {
    let mut command = util::command("git");
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    command
}

fn git_text(command: &mut Command) -> Res<String> {
    let out = output(command)?;
    if !out.status.success() {
        bail!(
            "{} failed: {}",
            util::describe(command),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn git_text_with_input(command: &mut Command, input: &[u8]) -> Res<String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| util::spawn_error(command, error))?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input)?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "{} failed: {}",
            util::describe(command),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn short(revision: &str) -> &str {
    &revision[..revision.len().min(10)]
}

fn clock(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn megabytes(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / 1_048_576.0)
}
