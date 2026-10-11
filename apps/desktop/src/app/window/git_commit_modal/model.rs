//! The commit review's data: the review draft gxserver's Git menu opens it with, one file's
//! diff draft, and the changed-files tree (`packages/core-ui/changed-files-tree-utils.ts` (deleted 2026-10-01)).
use serde_json::Value;
use std::cmp::Ordering;

/// One `changedFiles[]` row (`SidebarGitChangedFile`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitChangedFile {
    pub(crate) path: String,
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
}

/// `ChangedFilesTreeStat`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GitFileStat {
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
}

impl GitFileStat {
    pub(crate) fn is_zero(&self) -> bool {
        self.additions == 0 && self.deletions == 0
    }
}

/// `GitCommitModalDraft`, the `gitCommitDraft` of the `open` message
/// (`review_modal_draft` in packages/gx-core/src/git_menu/review.rs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitCommitDraft {
    pub(crate) request_id: String,
    /// `undefined` (no row), `null` (`(detached HEAD)`) or the branch name.
    pub(crate) branch: Option<Option<String>>,
    pub(crate) changed_files: Vec<GitChangedFile>,
    pub(crate) confirm_label: String,
    pub(crate) delete_worktree_after_default: bool,
    pub(crate) is_worktree: bool,
    pub(crate) show_commit_message: bool,
    pub(crate) suggested_subject: String,
    pub(crate) suggested_body: Option<String>,
    pub(crate) worktree_name: Option<String>,
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn count(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_f64().map(|n| n.max(0.0) as u64))
        })
        .unwrap_or(0)
}

impl GitCommitDraft {
    pub(crate) fn from_json(draft: &Value) -> Option<Self> {
        let request_id = text(draft, "requestId").filter(|id| !id.is_empty())?;
        let branch = match draft.get("branch") {
            None => None,
            Some(Value::String(branch)) => Some(Some(branch.clone())),
            Some(_) => Some(None),
        };
        let changed_files = draft
            .get("changedFiles")
            .and_then(Value::as_array)
            .map(|files| {
                files
                    .iter()
                    .filter_map(|file| {
                        Some(GitChangedFile {
                            path: text(file, "path")?,
                            additions: count(file, "additions"),
                            deletions: count(file, "deletions"),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let flag = |key: &str| draft.get(key).and_then(Value::as_bool);
        Some(Self {
            request_id,
            branch,
            changed_files,
            confirm_label: text(draft, "confirmLabel").unwrap_or_else(|| "Commit".to_string()),
            delete_worktree_after_default: flag("deleteWorktreeAfterDefault") == Some(true),
            is_worktree: flag("isWorktree") == Some(true),
            show_commit_message: flag("showCommitMessage").unwrap_or(true),
            suggested_subject: text(draft, "suggestedSubject").unwrap_or_default(),
            suggested_body: text(draft, "suggestedBody"),
            worktree_name: text(draft, "worktreeName"),
        })
    }

    /// `buildDraftMessage`: the trimmed subject, a blank line and the trimmed body when there is one.
    pub(crate) fn initial_message(&self) -> String {
        let subject = self.suggested_subject.trim();
        match self.suggested_body.as_deref().map(str::trim) {
            Some(body) if !body.is_empty() => format!("{subject}\n\n{body}"),
            _ => subject.to_string(),
        }
    }
}

/// `SidebarGitFileDiffDraft`: one file's patch, delivered as the `gitFileDiff` open message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitFileDiffDraft {
    pub(crate) file_path: String,
    pub(crate) patch: String,
    pub(crate) additions: Option<u64>,
    pub(crate) deletions: Option<u64>,
}

impl GitFileDiffDraft {
    pub(crate) fn from_json(draft: &Value) -> Option<Self> {
        let optional = |key: &str| {
            draft
                .get(key)
                .filter(|v| v.is_number())
                .map(|_| count(draft, key))
        };
        Some(Self {
            file_path: text(draft, "filePath")?,
            patch: text(draft, "patch").unwrap_or_default(),
            additions: optional("additions"),
            deletions: optional("deletions"),
        })
    }
}

/// `summarizeChangedFiles`.
pub(crate) fn summarize_changed_files<'a>(
    files: impl IntoIterator<Item = &'a GitChangedFile>,
) -> GitFileStat {
    files
        .into_iter()
        .fold(GitFileStat::default(), |total, file| GitFileStat {
            additions: total.additions + file.additions,
            deletions: total.deletions + file.deletions,
        })
}

/// `ChangedFilesTreeNode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ChangedFilesTreeNode {
    Directory {
        name: String,
        path: String,
        stat: GitFileStat,
        children: Vec<ChangedFilesTreeNode>,
    },
    File {
        name: String,
        path: String,
        stat: GitFileStat,
    },
}

impl ChangedFilesTreeNode {
    fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } | Self::File { name, .. } => name,
        }
    }
}

#[derive(Default)]
struct MutableDirectory {
    /// Insertion order, like the JS `Map`.
    directories: Vec<(String, MutableDirectory)>,
    files: Vec<ChangedFilesTreeNode>,
    name: String,
    path: String,
    stat: GitFileStat,
}

/// The ICU root collation order of ASCII punctuation and symbols, which all sort
/// before digits and letters. Unlisted characters keep their code point order after these.
const PUNCTUATION_ORDER: &str = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CollationKey {
    Punctuation(usize),
    Other(u32),
    Digits(String),
    Letter(String),
}

fn collation_keys(text: &str) -> Vec<CollationKey> {
    let mut keys = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            let mut digits = String::from(ch);
            while let Some(next) = chars.peek().copied().filter(char::is_ascii_digit) {
                digits.push(next);
                chars.next();
            }
            let trimmed = digits.trim_start_matches('0');
            // Numeric order: a longer run of significant digits is the larger number.
            keys.push(CollationKey::Digits(format!(
                "{:0>20}",
                if trimmed.is_empty() { "0" } else { trimmed }
            )));
        } else if ch.is_alphabetic() {
            keys.push(CollationKey::Letter(ch.to_lowercase().collect()));
        } else if let Some(rank) = PUNCTUATION_ORDER.find(ch) {
            keys.push(CollationKey::Punctuation(rank));
        } else {
            keys.push(CollationKey::Other(ch as u32));
        }
    }
    keys
}

/// `localeCompare(right, undefined, { numeric: true, sensitivity: 'base' })`: case-insensitive,
/// digit runs compared as numbers, punctuation before digits before letters.
pub(crate) fn compare_by_name(left: &str, right: &str) -> Ordering {
    collation_keys(left).cmp(&collation_keys(right))
}

fn compact_directory_node(node: ChangedFilesTreeNode) -> ChangedFilesTreeNode {
    let ChangedFilesTreeNode::Directory {
        name,
        path,
        stat,
        children,
    } = node
    else {
        return node;
    };
    let children: Vec<_> = children.into_iter().map(compact_directory_node).collect();
    let (mut name, mut path, mut stat, mut children) = (name, path, stat, children);
    while children.len() == 1 && matches!(children[0], ChangedFilesTreeNode::Directory { .. }) {
        let Some(ChangedFilesTreeNode::Directory {
            name: child_name,
            path: child_path,
            stat: child_stat,
            children: grandchildren,
        }) = children.pop()
        else {
            break;
        };
        name = format!("{name}/{child_name}");
        path = child_path;
        stat = child_stat;
        children = grandchildren;
    }
    ChangedFilesTreeNode::Directory {
        name,
        path,
        stat,
        children,
    }
}

fn to_tree_nodes(directory: MutableDirectory) -> Vec<ChangedFilesTreeNode> {
    let mut subdirectories = directory.directories;
    subdirectories.sort_by(|left, right| compare_by_name(&left.1.name, &right.1.name));
    let mut nodes: Vec<ChangedFilesTreeNode> = subdirectories
        .into_iter()
        .map(|(_, subdirectory)| {
            let name = subdirectory.name.clone();
            let path = subdirectory.path.clone();
            let stat = subdirectory.stat;
            compact_directory_node(ChangedFilesTreeNode::Directory {
                name,
                path,
                stat,
                children: to_tree_nodes(subdirectory),
            })
        })
        .collect();
    let mut files = directory.files;
    files.sort_by(|left, right| compare_by_name(left.name(), right.name()));
    nodes.extend(files);
    nodes
}

/// `buildChangedFilesTree`.
///
/// CDXC:Worktrees 2026-05-18-23:07:
/// Git review surfaces show changed files as a compact directory tree with aggregated additions/deletions so worktree commit/PR flows can select files without flattening large project paths.
pub(crate) fn build_changed_files_tree(files: &[GitChangedFile]) -> Vec<ChangedFilesTreeNode> {
    let mut root = MutableDirectory::default();
    for file in files {
        let segments: Vec<&str> = file
            .path
            .split(['/', '\\'])
            .filter(|segment| !segment.is_empty())
            .collect();
        let Some((file_name, parents)) = segments.split_last() else {
            continue;
        };
        let stat = GitFileStat {
            additions: file.additions,
            deletions: file.deletions,
        };
        root.stat.additions += stat.additions;
        root.stat.deletions += stat.deletions;
        let mut current = &mut root;
        for segment in parents {
            let next_path = if current.path.is_empty() {
                segment.to_string()
            } else {
                format!("{}/{segment}", current.path)
            };
            let index = match current
                .directories
                .iter()
                .position(|(name, _)| name == segment)
            {
                Some(index) => index,
                None => {
                    current.directories.push((
                        segment.to_string(),
                        MutableDirectory {
                            name: segment.to_string(),
                            path: next_path,
                            ..MutableDirectory::default()
                        },
                    ));
                    current.directories.len() - 1
                }
            };
            current = &mut current.directories[index].1;
            current.stat.additions += stat.additions;
            current.stat.deletions += stat.deletions;
        }
        current.files.push(ChangedFilesTreeNode::File {
            name: file_name.to_string(),
            path: segments.join("/"),
            stat,
        });
    }
    to_tree_nodes(root)
}

/// One drawn row of the tree, flattened for the virtual list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChangedFilesTreeRow {
    pub(crate) depth: usize,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) stat: GitFileStat,
    /// `Some(expanded)` for a directory.
    pub(crate) directory: Option<bool>,
    /// The first row of a top-level node after the first: `.changed-files-tree`'s grid gap sits above it.
    pub(crate) top_level_gap: bool,
}

/// The rows `ChangedFilesTree` renders: every directory expanded unless the user folded it.
pub(crate) fn flatten_changed_files_tree(
    nodes: &[ChangedFilesTreeNode],
    collapsed: &std::collections::HashSet<String>,
) -> Vec<ChangedFilesTreeRow> {
    fn walk(
        nodes: &[ChangedFilesTreeNode],
        depth: usize,
        collapsed: &std::collections::HashSet<String>,
        rows: &mut Vec<ChangedFilesTreeRow>,
    ) {
        for node in nodes {
            let top_level_gap = depth == 0 && !rows.is_empty();
            match node {
                ChangedFilesTreeNode::Directory {
                    name,
                    path,
                    stat,
                    children,
                } => {
                    let expanded = !collapsed.contains(path);
                    rows.push(ChangedFilesTreeRow {
                        depth,
                        name: name.clone(),
                        path: path.clone(),
                        stat: *stat,
                        directory: Some(expanded),
                        top_level_gap,
                    });
                    if expanded {
                        walk(children, depth + 1, collapsed, rows);
                    }
                }
                ChangedFilesTreeNode::File { name, path, stat } => {
                    rows.push(ChangedFilesTreeRow {
                        depth,
                        name: name.clone(),
                        path: path.clone(),
                        stat: *stat,
                        directory: None,
                        top_level_gap,
                    });
                }
            }
        }
    }
    let mut rows = Vec::new();
    walk(nodes, 0, collapsed, &mut rows);
    rows
}
