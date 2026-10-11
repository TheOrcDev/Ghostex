//! The feed as gxserver sends it (`server/src/bot_feed.rs`, one entry per cron job) and the rows
//! the page draws from it. Which jobs sit in which group, which channels show and in what order,
//! and which runs a channel shows are `ghostex_gx_core::bot_feed`'s rules.

use chrono::NaiveDate;
use ghostex_gx_core::bot_feed::{FeedChannel, FeedJob, FeedRun, job_key};
use gpui::SharedString;
use serde::Deserialize;
use serde_json::Value;

/// One cron job and its delivered runs.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BotFeedJob {
    pub(crate) job_id: String,
    pub(crate) name: String,
    pub(crate) profile: String,
    pub(crate) bot_name: SharedString,
    pub(crate) schedule: Option<String>,
    /// Oldest first.
    pub(crate) runs: Vec<BotFeedRun>,
    /// [`job_key`]. Set by [`parse_feed`].
    #[serde(skip)]
    pub(crate) key: String,
}

impl FeedJob for BotFeedJob {
    fn key(&self) -> &str {
        &self.key
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn profile(&self) -> &str {
        &self.profile
    }
    fn run_count(&self) -> usize {
        self.runs.len()
    }
    fn run_at(&self, run: usize) -> &str {
        &self.runs[run].run_at
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BotFeedRun {
    /// `YYYY-MM-DD HH:MM:SS`, Hermes' own clock.
    pub(crate) run_at: String,
    pub(crate) body: SharedString,
    pub(crate) failed: bool,
    /// The run's Markdown view id. Set by [`parse_feed`].
    #[serde(skip)]
    pub(crate) id: SharedString,
}

#[derive(Deserialize)]
struct BotFeed {
    channels: Vec<BotFeedJob>,
}

/// The jobs in gxserver's order, newest activity first.
pub(crate) fn parse_feed(result: Value) -> Result<Vec<BotFeedJob>, String> {
    let mut jobs = serde_json::from_value::<BotFeed>(result)
        .map_err(|error| format!("The feed could not be read: {error}"))?
        .channels;
    for job in &mut jobs {
        job.key = job_key(&job.profile, &job.job_id);
        for run in &mut job.runs {
            run.id = format!("bot-feed-run-{}-{}", job.key, run.run_at).into();
        }
    }
    Ok(jobs)
}

/// A channel's name the way Discord writes it: lowercase words joined by hyphens, or its key for
/// a name with no words.
pub(crate) fn channel_slug(channel: &FeedChannel) -> SharedString {
    let slug = channel
        .name
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        channel.key.clone().into()
    } else {
        slug.into()
    }
}

/// Every bot with a job, as `(profile, bot name)` in first-seen order, for the bot filter.
pub(crate) fn feed_bots(jobs: &[BotFeedJob]) -> Vec<(String, SharedString)> {
    let mut bots: Vec<(String, SharedString)> = Vec::new();
    for job in jobs {
        if !bots.iter().any(|(profile, _)| profile == &job.profile) {
            bots.push((job.profile.clone(), job.bot_name.clone()));
        }
    }
    bots
}

/// One row of the message list. A day separator carries its `YYYY-MM-DD` as well as its label,
/// since a label repeats (the same weekday and date in another year) and "Today" moves at midnight.
pub(crate) enum FeedRow {
    Day { date: String, label: String },
    Run(FeedRun),
}

impl FeedRow {
    /// What stays the same for this row across a reload, so the list can keep its place.
    pub(crate) fn key(&self, jobs: &[BotFeedJob]) -> SharedString {
        match self {
            FeedRow::Day { date, .. } => date.clone().into(),
            FeedRow::Run(run) => jobs[run.job].runs[run.run].id.clone(),
        }
    }
}

/// The runs a view shows, oldest first, with a day separator wherever the date changes.
pub(crate) fn feed_rows(jobs: &[BotFeedJob], runs: Vec<FeedRun>, today: NaiveDate) -> Vec<FeedRow> {
    let mut rows = Vec::with_capacity(runs.len());
    let mut day = None;
    for run in runs {
        let date = jobs[run.job].runs[run.run]
            .run_at
            .get(..10)
            .unwrap_or_default();
        if day != Some(date) {
            day = Some(date);
            rows.push(FeedRow::Day {
                date: date.to_string(),
                label: day_label(date, today),
            });
        }
        rows.push(FeedRow::Run(run));
    }
    rows
}

/// "Today", "Yesterday", or the weekday and date of a `YYYY-MM-DD`.
fn day_label(date: &str, today: NaiveDate) -> String {
    let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return date.to_string();
    };
    if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else {
        date.format("%A, %B %-d").to_string()
    }
}

/// `HH:MM` of a run time.
pub(crate) fn run_clock(run_at: &str) -> &str {
    run_at.get(11..16).unwrap_or(run_at)
}
