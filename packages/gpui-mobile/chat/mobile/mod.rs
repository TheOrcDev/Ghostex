//! The API a phone host embeds.
//!
//! ```ignore
//! Application::with_platform(platform)
//!     .with_assets(ghostex_gpui_mobile_chat::asset_source())
//!     .run_embedded(|cx| {
//!         ghostex_gpui_mobile_chat::init(cx, ChatInit { data_dir, base_url, auth_token, ..Default::default() });
//!         cx.open_window(options, |window, cx| {
//!             let transcript = ghostex_gpui_mobile_chat::open_transcript(window, cx, project_id, session_id);
//!             cx.subscribe(&transcript, |_, event: &ChatTranscriptEvent, cx| { /* to React Native */ }).detach();
//!             ghostex_gpui_mobile_chat::root_view(transcript, window, cx)
//!         });
//!     });
//! ```
//!
//! - [`init`] once per process ([`ChatInit`]); [`set_endpoint`], [`set_light_appearance`],
//!   [`install_settings`] and [`set_clipboard_handler`] for later changes.
//! - [`open_transcript`] gives the root view, [`ChatTranscript`]: `open_session` switches sessions,
//!   `send_text` sends through the core's own send path, `set_draft` / `save_draft` keep the core's
//!   draft in step with the host's composer, `dispatch_action` performs any other core action.
//! - [`ChatTranscriptEvent`] is everything the view asks of its host: [`ComposerSummary`] on
//!   change, host actions, toasts, and (opt-in) the whole document.
pub(crate) mod gxserver_http;
mod init;
mod summary;
mod transcript;

pub use init::{
    ChatInit, init, install_settings, set_clipboard_handler, set_endpoint, set_light_appearance,
};
pub(crate) use init::{
    Endpoint, copy_to_host_clipboard, data_dir, light_appearance, reduce_motion,
};
pub use summary::ComposerSummary;
pub use transcript::{
    ChatTranscript, ChatTranscriptEvent, SendMode, SendOutcome, SessionRef, open_transcript,
    root_view,
};
