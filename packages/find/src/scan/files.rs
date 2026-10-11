use std::fs;
use std::path::{Path, PathBuf};

pub(super) const MAX_HISTORY_FILE_BYTES: u64 = 64 * 1024 * 1024;

pub(super) struct SourceStamp {
    pub(super) size_text: String,
    pub(super) mtime_text: String,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

pub(super) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

pub(super) fn read_dir_sorted(base: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

pub(super) fn read_all(path: &Path) -> Option<Vec<u8>> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_HISTORY_FILE_BYTES {
        return None;
    }
    fs::read(path).ok()
}

pub(super) fn read_prefix(path: &Path, limit: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = fs::File::open(path).ok()?;
    let mut buf = Vec::with_capacity(limit.min(64 * 1024));
    file.take(limit as u64).read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub(super) fn mtime_seconds(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn mtime_nanos(meta: &fs::Metadata) -> Option<u128> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
}

pub(super) fn stamp_from_stat(meta: &fs::Metadata) -> Option<SourceStamp> {
    Some(SourceStamp {
        size_text: meta.len().to_string(),
        mtime_text: mtime_nanos(meta)?.to_string(),
    })
}
