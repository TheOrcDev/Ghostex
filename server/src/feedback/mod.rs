//! In-app feedback: the issue draft a client shows the user for review, and the send that posts
//! the reviewed issue to the Ghostex feedback relay, which files it as a GitHub issue.

mod draft;
mod http;
mod relay;

pub(crate) use http::handle_feedback_http;
