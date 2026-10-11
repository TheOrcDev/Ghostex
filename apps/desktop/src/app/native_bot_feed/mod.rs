//! The native GPUI Bot automations feed: a Discord-style page with a channel per Hermes cron job,
//! groups the user makes to read several jobs together, and each run as a message, opened from the
//! Bots sidebar's Automations row. Desktop only, like Automate: the GPUI web build does not compile
//! it.
//! SEE-ALSO: server/src/bot_feed.rs (`/api/listBotFeed`, the runs it reads and the ones it hides),
//! app/native_sidebar/automations_row.rs (the row that opens it).
mod host;
mod model;
mod storage;
mod view;

pub(crate) use view::NativeBotFeedView;
