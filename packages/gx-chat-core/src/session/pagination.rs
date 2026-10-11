//! How far back the history window reaches, and whether there is more of it.
//!
//! Ported from `packages/core-ui/chat/session-chat-pagination.ts`.

use crate::session::constants::PAGE;

/// The window one "Load earlier" asks for on top of the current one.
pub fn next_limit(current: u32) -> u32 {
    current + PAGE
}

/// Client heuristic after a read: a full window implies more history. The server's exact
/// `hasMore` is preferred whenever it is present.
pub fn has_more_history(returned_count: usize, requested_limit: u32) -> bool {
    returned_count >= requested_limit as usize
}

/// One page's boundary facts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageBoundary {
    pub message_count: usize,
    pub has_more: bool,
    /// The capability probe gxserver added when it began filtering before counting.
    pub has_more_exact: Option<bool>,
    pub before_offset: u64,
}

/// `sessionChatPageHasMore`: trust a current daemon's exact boundary.
///
/// For an older daemon, keep one pagination probe available while its byte cursor is still moving
/// backwards; the probe disappears as soon as a read makes no progress, so a bad legacy
/// `hasMore: false` cannot strand history or cause an endless request loop.
pub fn page_has_more(page: PageBoundary, requested_before_offset: Option<u64>) -> bool {
    if page.has_more || page.has_more_exact == Some(true) {
        return page.has_more;
    }
    if page.message_count == 0 || page.before_offset == 0 {
        return false;
    }
    match requested_before_offset {
        None => true,
        Some(requested) => page.before_offset < requested,
    }
}
