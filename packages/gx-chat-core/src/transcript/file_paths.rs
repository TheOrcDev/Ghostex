//! Which strings in a chat message are file references, decided once for every renderer.
//!
//! Ported from `packages/shared/session-chat-presentation/file-paths.ts`. The deliberately high bar
//! for saying "yes" is the rule itself: a path that becomes a chip in React must become a pill in
//! the native transcript, and ordinary code such as `cargo check` must become neither.
//!
//! CDXC:SessionChat 2026-09-18 SEE-ALSO:
//! The GPUI transcript consumes these through native_markdown.rs; React consumes them through
//! session-chat-file-paths.ts (deleted 2026-10-01) and session-chat-code-fence-meta.ts (deleted 2026-10-01).

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::transcript::file_position::{split_file_position, FilePosition};
use crate::transcript::jsstr::{ascii_lower, is_js_space, js_trim, utf16_len};

/// Long enough for any real path; past this it is a blob, not a reference.
const MAX_CANDIDATE_LENGTH: usize = 240;

/// Conventional filenames that carry no extension. Any other extensionless basename stays plain:
/// `src/utils`, `origin/main`, and `text/plain` are all shaped like paths and none of them is one.
pub fn is_extensionless_file_name(basename: &str) -> bool {
    matches!(
        basename,
        "AUTHORS"
            | "BUILD"
            | "Brewfile"
            | "CHANGELOG"
            | "CODEOWNERS"
            | "COPYING"
            | "Caddyfile"
            | "Containerfile"
            | "Dockerfile"
            | "Fastfile"
            | "GNUmakefile"
            | "Gemfile"
            | "Jenkinsfile"
            | "Justfile"
            | "LICENCE"
            | "LICENSE"
            | "Makefile"
            | "NOTICE"
            | "Podfile"
            | "Procfile"
            | "README"
            | "Rakefile"
            | "Vagrantfile"
            | "WORKSPACE"
            | "justfile"
            | "makefile"
    )
}

/// Enough of a generic-TLD list to catch a bare hostname written without a scheme. Country codes
/// are deliberately absent: `.pl`, `.pt`, `.es`, and `.in` are all real file extensions, and
/// refusing them would cost more real paths than the fake hostnames it would save.
fn is_hostname_tld(label: &str) -> bool {
    matches!(
        label,
        "ai" | "app"
            | "biz"
            | "cloud"
            | "co"
            | "com"
            | "dev"
            | "edu"
            | "gov"
            | "info"
            | "io"
            | "net"
            | "org"
            | "xyz"
    )
}

/// `^\d+(?:\.\d+)+$`.
fn is_dotted_number(segment: &str) -> bool {
    let parts: Vec<&str> = segment.split('.').collect();
    parts.len() > 1
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// `example.com`, `localhost`, `127.0.0.1`, `1.2.3`: a host or a version.
fn looks_like_host_or_version(segment: &str) -> bool {
    if segment == "localhost" || is_dotted_number(segment) {
        return true;
    }
    let lower = segment.to_lowercase();
    let labels: Vec<&str> = lower.split('.').collect();
    labels.len() > 1 && labels.last().is_some_and(|last| is_hostname_tld(last))
}

/// `^[A-Za-z0-9._+@~-]+$`.
fn is_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'@' | b'~' | b'-')
        })
}

/// `\.[A-Za-z][A-Za-z0-9_+-]*$`: a file type starts with a letter. `.ts`, `.rs`, `.zshrc` are
/// extensions; `.2` in `v1.2` and `.9` in `p99.9` are version fragments.
fn has_letter_extension(basename: &str) -> bool {
    let Some(dot) = basename.rfind('.') else {
        return false;
    };
    let extension = &basename[dot + 1..];
    extension
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && extension
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
}

/// One resolved file reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePathRef {
    /// Final path segment. Only the icon and the layout use it.
    pub basename: String,
    /// The path as the agent wrote it, minus its line, range, or column suffix.
    pub path: String,
    pub position: Option<FilePosition>,
}

fn is_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
        && matches!(bytes.get(2), Some(b'\\' | b'/')))
        || path.starts_with("\\\\")
}

fn resolve_file_path_reference(text: &str, require_path_evidence: bool) -> Option<FilePathRef> {
    let trimmed = js_trim(text);
    if trimmed.is_empty() || utf16_len(trimmed) > MAX_CANDIDATE_LENGTH {
        return None;
    }
    // Whitespace or a backtick means this span holds more than one token.
    if trimmed
        .chars()
        .any(|character| is_js_space(character) || character == '`')
    {
        return None;
    }

    let (path, position) = split_file_position(trimmed);
    if path.is_empty() {
        return None;
    }

    let windows = is_windows_path(path);
    // Backslashes only separate directories on a path that announced itself as a Windows path;
    // anywhere else a backslash is an escape, and the segment check below rejects it.
    let normalized = if windows {
        path.replace('\\', "/")
    } else {
        path.to_string()
    };
    // The drive letter and the UNC leader carry a colon and a doubled slash that no segment may
    // contain, so they are peeled off before the segment check.
    let bytes = normalized.as_bytes();
    let drive = bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
    let body = if windows && (drive || normalized.starts_with("//")) {
        normalized[2..].to_string()
    } else {
        normalized.clone()
    };

    let segments: Vec<&str> = body
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let basename = (*segments.last()?).to_string();
    if segments.iter().any(|segment| !is_path_segment(segment)) {
        return None;
    }

    let announces_itself = windows
        || normalized.starts_with('/')
        || normalized.starts_with("~/")
        || normalized.starts_with("./")
        || normalized.starts_with("../");
    let has_separator = announces_itself || segments.len() > 1;
    if require_path_evidence && !has_separator && position.is_none() {
        return None;
    }

    if !announces_itself
        && segments
            .first()
            .is_some_and(|first| looks_like_host_or_version(first))
    {
        return None;
    }

    if !has_letter_extension(&basename) && !is_extensionless_file_name(&basename) {
        return None;
    }

    Some(FilePathRef {
        basename,
        path: path.to_string(),
        position,
    })
}

/// Decides whether one inline-code span is a file reference.
pub fn resolve_inline_code_file_path(text: &str) -> Option<FilePathRef> {
    resolve_file_path_reference(text, true)
}

/// The same decision for the title a fenced code block names.
///
/// One clause is dropped, and only one: the demand that a bare word carry a separator or a `:line`
/// before it counts as a path. A fence title is not ambiguous: the fence says "this block is that
/// file", which is why the header already draws a file glyph beside it.
pub fn resolve_fence_title_file_path(title: &str) -> Option<FilePathRef> {
    resolve_file_path_reference(title, false)
}

fn markdown_extensions() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        ["markdown", "md", "mdown", "mdx", "mkdn", "rst"]
            .into_iter()
            .collect()
    })
}

fn code_extensions() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "bash", "c", "cc", "cjs", "cpp", "cs", "css", "dart", "ex", "exs", "fish", "go",
            "gradle", "h", "hpp", "hs", "html", "java", "js", "json", "jsonc", "jsx", "kt", "kts",
            "lua", "m", "mjs", "mm", "php", "pl", "py", "rb", "rs", "scala", "scss", "sh", "sql",
            "svelte", "swift", "toml", "ts", "tsx", "vue", "xml", "yaml", "yml", "zig", "zsh",
        ]
        .into_iter()
        .collect()
    })
}

/// Ghostex has no file-type icon set of its own, so the chip picks from the house icon set. Three
/// glyphs is the whole vocabulary: prose, source, and everything else.
pub fn file_path_icon_name(basename: &str) -> &'static str {
    let extension = match basename.rfind('.') {
        Some(dot) => ascii_lower(&basename[dot + 1..]),
        None => ascii_lower(basename),
    };
    if markdown_extensions().contains(extension.as_str()) {
        return "markdown";
    }
    if code_extensions().contains(extension.as_str()) || is_extensionless_file_name(basename) {
        return "file-code";
    }
    "file"
}

/// `(?:^|\s)(?:title|file(?:name)?)=(?:"([^"]+)"|'([^']+)'|(\S+))` case-insensitively.
fn fence_title_attribute(meta: &str) -> Option<&str> {
    for (start, _) in meta.char_indices() {
        if start > 0 && !meta[..start].chars().next_back().is_some_and(is_js_space) {
            continue;
        }
        let rest = &meta[start..];
        let Some(name) = ["title=", "filename=", "file="]
            .into_iter()
            .find(|name| rest.len() >= name.len() && rest[..name.len()].eq_ignore_ascii_case(name))
        else {
            continue;
        };
        let value = &rest[name.len()..];
        for quote in ['"', '\''] {
            if let Some(body) = value.strip_prefix(quote) {
                if let Some(end) = body.find(quote) {
                    if end > 0 {
                        return Some(&body[..end]);
                    }
                }
            }
        }
        let bare: usize = value
            .chars()
            .take_while(|character| !is_js_space(*character))
            .map(char::len_utf8)
            .sum();
        if bare > 0 {
            return Some(&value[..bare]);
        }
    }
    None
}

/// `^[\w@][\w@./-]*\.[A-Za-z0-9]+$`: a bare token only counts as a filename when it reads like one.
fn is_fence_filename_token(token: &str) -> bool {
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = token.as_bytes();
    if !bytes
        .first()
        .is_some_and(|byte| is_word(*byte) || *byte == b'@')
    {
        return false;
    }
    let Some(dot) = token.rfind('.') else {
        return false;
    };
    let extension = &token[dot + 1..];
    if extension.is_empty() || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return false;
    }
    token[..dot]
        .bytes()
        .all(|byte| is_word(byte) || matches!(byte, b'@' | b'.' | b'/' | b'-'))
}

/// The filename this fence names, or `None` when it names none.
///
/// Fence meta is everything an agent writes after the language on a ``` line, and agents use all
/// four spellings: `title="src/main.ts"`, `file=src/main.ts`, `filename=src/main.ts`, and a bare
/// `src/main.ts`.
pub fn fence_title(meta: Option<&str>) -> Option<&str> {
    let meta = meta?;
    if let Some(named) = fence_title_attribute(meta) {
        return Some(named);
    }
    meta.split(is_js_space)
        .find(|token| is_fence_filename_token(token))
}

/// CDXC:SessionChat 2026-09-06 DECISION:
/// User: @file mentions must follow all the same chat rules as [File #N](path) references, including images and other files. An explicit mention becomes a link like any other file reference, so previews, file opening, positions and context menus all use the attachment renderer; the @ marker is never part of the destination.
/// One path found in ordinary prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BareFilePath {
    /// Byte offset of the candidate inside the scanned text.
    pub start: usize,
    pub end: usize,
    /// The path the renderer opens: an `@mention` has already lost its marker.
    pub path: String,
    /// True when the author asked for it with `@`, rather than it being spotted in prose.
    pub mention: bool,
}

/// `[([{'"<]*@"[^\r\n]+?"(?=$|[\s)\]},.!?;'">])|\S+`, one match at a time.
///
/// Returns the byte range of the whole token.
fn next_prose_token(text: &str, from: usize) -> Option<(usize, usize)> {
    let mut start = from;
    while start < text.len() {
        let character = text[start..].chars().next()?;
        if !is_js_space(character) {
            break;
        }
        start += character.len_utf8();
    }
    if start >= text.len() {
        return None;
    }
    // The quoted-mention alternative comes first and wins wherever it matches.
    let leading: usize = text[start..]
        .chars()
        .take_while(|character| "([{'\"<".contains(*character))
        .map(char::len_utf8)
        .sum();
    if text[start + leading..].starts_with("@\"") {
        let body_start = start + leading + 2;
        let mut at = body_start;
        while let Some(quote) = text[at..].find('"') {
            let close = at + quote;
            if text[body_start..close].contains(['\r', '\n']) {
                break;
            }
            // `[^\r\n]+?` is lazy but needs at least one character.
            if close > body_start {
                let after = text[close + 1..].chars().next();
                if after.is_none_or(|character| {
                    is_js_space(character) || ")]},.!?;'\">".contains(character)
                }) {
                    return Some((start, close + 1));
                }
            }
            at = close + 1;
        }
    }
    let run: usize = text[start..]
        .chars()
        .take_while(|character| !is_js_space(*character))
        .map(char::len_utf8)
        .sum();
    Some((start, start + run))
}

/// Finds the paths a person typed into a chat message without marking them up:
/// `packages/shared/x.ts:12`, `@src/main.rs`, `@"my notes/plan.md"`.
///
/// Candidate discovery only splits on whitespace and sentence punctuation;
/// [`resolve_inline_code_file_path`] still makes every implicit path / not-path decision. Callers
/// are responsible for skipping text that is already a link, code, or HTML.
pub fn bare_file_paths(text: &str) -> Vec<BareFilePath> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some((token_start, token_end)) = next_prose_token(text, cursor) {
        cursor = token_end.max(token_start + 1);
        let raw_token = &text[token_start..token_end];
        let leading: usize = raw_token
            .chars()
            .take_while(|character| "([{'\"<".contains(*character))
            .map(char::len_utf8)
            .sum();
        let without_leading = &raw_token[leading..];
        let quoted_mention = without_leading.starts_with("@\"") && without_leading.ends_with('"');
        let mut trailing = if quoted_mention {
            0
        } else {
            without_leading
                .chars()
                .rev()
                .take_while(|character| ")]},.!?;'\">".contains(*character))
                .map(char::len_utf8)
                .sum()
        };
        if without_leading.starts_with('@') && !quoted_mention {
            // Keep closing delimiters owned by the filename, such as @report(final).pdf or
            // @reports/(final).
            while trailing > 0 {
                let closing = without_leading[without_leading.len() - trailing..]
                    .chars()
                    .next();
                let opening = match closing {
                    Some(')') => Some('('),
                    Some(']') => Some('['),
                    Some('}') => Some('{'),
                    _ => None,
                };
                let (Some(opening), Some(closing)) = (opening, closing) else {
                    break;
                };
                let kept = &without_leading[..without_leading.len() - trailing];
                if kept.matches(opening).count() <= kept.matches(closing).count() {
                    break;
                }
                trailing -= closing.len_utf8();
            }
        }
        let candidate = &without_leading[..without_leading.len() - trailing];
        let mention_path = if quoted_mention {
            // `candidate.slice(2, -1)`, which CLAMPS: the two-character token `@"` satisfies both
            // `startsWith('@"')` and `endsWith('"')` on the same quote, and JavaScript answers the
            // empty string where `&candidate[2..1]` panics. A user typing `did you mean @" ?` took
            // the chat window down (2026-09-22).
            candidate
                .get(2..candidate.len().saturating_sub(1))
                .unwrap_or("")
        } else if candidate.starts_with('@') && !candidate.starts_with("@\"") {
            &candidate[1..]
        } else {
            ""
        };
        let reference = if mention_path.is_empty() {
            resolve_inline_code_file_path(candidate)
        } else {
            None
        };
        if !mention_path.is_empty() || reference.is_some() {
            let start = token_start + leading;
            found.push(BareFilePath {
                start,
                end: start + candidate.len(),
                path: if mention_path.is_empty() {
                    candidate.to_string()
                } else {
                    mention_path.to_string()
                },
                mention: !mention_path.is_empty(),
            });
        }
    }
    found
}
