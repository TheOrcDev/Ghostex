//! The Bot automations feed's channel rules: which cron jobs sit in which group, which channels the
//! list shows and in what order, and which runs a channel shows. The desktop's feed page
//! (apps/desktop/src/app/native_bot_feed/) draws what these answer and saves [`FeedPrefs`].
//!
//! SEE-ALSO: server/src/bot_feed.rs (`/api/listBotFeed`, one entry per cron job),
//! packages/client-storage/catalog.ts (`botFeed`, where [`FeedPrefs`] is saved).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The prefix of a group's key. A job's own channel is keyed by the job (`profile/jobId`), which
/// always holds a `/`, so the two kinds of key never meet. Saved groups keep the `channel:` form
/// they were first saved with.
const GROUP_KEY_PREFIX: &str = "channel:";
/// Discord's own cap on a channel name.
const MAX_GROUP_NAME_CHARS: usize = 100;

/// A job's key: `profile/jobId`, since a job id is unique only within its profile.
pub fn job_key(profile: &str, job_id: &str) -> String {
    format!("{profile}/{job_id}")
}

/// One cron job as gxserver sends it, whatever type the host keeps it in.
pub trait FeedJob {
    /// [`job_key`].
    fn key(&self) -> &str;
    fn name(&self) -> &str;
    fn profile(&self) -> &str;
    fn run_count(&self) -> usize;
    /// `YYYY-MM-DD HH:MM:SS`, Hermes' own clock, oldest run first.
    fn run_at(&self, run: usize) -> &str;
}

/// How the channel list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FeedSort {
    /// The channel that ran most recently first, gxserver's own order.
    #[default]
    Activity,
    /// The order the user dragged the channels into.
    Manual,
}

/// What the feed remembers between launches, as one client-storage value. New fields take their
/// default, so an older value keeps reading.
///
/// CDXC:Bots 2026-09-26 DECISION:
/// User: the bot filter narrows the channel list and `#all` to one bot; Manual sorting keeps one order for every channel that the filter only hides, a new job lands at the bottom, and `#all` stays first; the sort mode, the order and the filter survive a restart.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedPrefs {
    pub sort: FeedSort,
    /// Channel keys in the hand-made order. A key whose job is gone stays harmlessly, so a job that
    /// is hidden for a while (no delivered run) comes back where it was.
    pub order: Vec<String>,
    /// The profile the feed is narrowed to; `None` is every bot.
    pub bot: Option<String>,
    /// The groups the user made, in the order they were made. Saved as `channels`, the name the
    /// first saved values used.
    #[serde(rename = "channels")]
    pub groups: Vec<FeedGroup>,
    /// The keys of the groups drawn collapsed.
    pub collapsed: Vec<String>,
    /// Per job key, the time of the newest run read; a job with none counts every run unread.
    ///
    /// CDXC:Bots 2026-09-27 DECISION:
    /// User: a group shows how many jobs it holds and how many unread runs, and each job's channel its own unread runs. Only opening a group or a job's own channel reads its runs, never `#all`, like a Discord channel's unread; the counts start at zero on the first launch that has them.
    pub read: BTreeMap<String, String>,
    /// CDXC:Bots 2026-09-27 DECISION:
    /// User: the feed ships with `#all`, and it can be hidden (right-click it) and shown again (right-click the Feeds title); with it hidden the feed opens on the first channel in the list.
    pub hide_all: bool,
}

/// A group the user made, and the jobs moved into it.
///
/// CDXC:Bots 2026-09-27 DECISION:
/// User: group cron jobs in Ghostex's own feed, the way Discord does, with no Discord or Telegram and no Hermes change: a + New group button under the channel list makes an empty named group, and clicking a group shows every run of its jobs the way `#all` shows every job's.
/// A job sits in one group at a time, drawn indented under it and nowhere else in the list, and its own channel still opens just that job; job names come from the Hermes config and are never renamed, while a group can be renamed, collapsed and deleted (its jobs go back below the groups), and stays until it is deleted.
/// Supersedes the same day's Move to channel rule, where a job left its own channel for a made channel and a channel left with no jobs disappeared. The grouping is saved with the rest of the feed's state in client storage, since only this app's feed draws groups and gxserver keeps answering one entry per job.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedGroup {
    /// `channel:<n>`.
    pub key: String,
    pub name: String,
    /// Job keys, in the order they joined. A job gone from Hermes stays until it is moved.
    pub jobs: Vec<String>,
}

/// Where a channel sits in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedChannelKind {
    /// A group the user made, holding the jobs moved into it.
    Group,
    /// One job's own channel, under the group keyed here while the job is in one.
    Job { group: Option<String> },
}

/// A channel the list can draw: a group with the jobs in it that gxserver sent, or one job's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedChannel {
    pub key: String,
    pub name: String,
    /// Indexes into the jobs, newest activity first.
    pub jobs: Vec<usize>,
    pub kind: FeedChannelKind,
}

/// One run a view shows: `channels[channel]` is the channel it is drawn under (the job's own in
/// `#all`), and it is `jobs[job]`'s `run`th run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeedRun {
    pub channel: usize,
    pub job: usize,
    pub run: usize,
}

impl FeedPrefs {
    /// Whether the bot filter lets this bot's jobs show.
    pub fn shows_bot(&self, profile: &str) -> bool {
        self.bot.as_deref().is_none_or(|bot| bot == profile)
    }

    /// Appends the channels the manual order has not placed yet, in the order they arrived, so a
    /// new job lands at the bottom and stays there. True when the order changed.
    pub fn place_new_channels(&mut self, channels: &[FeedChannel]) -> bool {
        let before = self.order.len();
        for channel in channels {
            if !self.order.contains(&channel.key) {
                self.order.push(channel.key.clone());
            }
        }
        self.order.len() != before
    }

    /// Puts the channel keyed `moved` where `target` is: after it when dragged down, before it when
    /// dragged up. Only channels in the same section trade places: two groups, two jobs in one
    /// group, or two jobs in none. True when the order changed.
    pub fn move_channel(&mut self, moved: &str, target: &str, channels: &[FeedChannel]) -> bool {
        let kind = |key: &str| {
            channels
                .iter()
                .find(|channel| channel.key == key)
                .map(|channel| &channel.kind)
        };
        if kind(moved).is_none() || kind(moved) != kind(target) {
            return false;
        }
        let position = |order: &[String], key: &str| order.iter().position(|item| item == key);
        let (Some(from), Some(to)) = (position(&self.order, moved), position(&self.order, target))
        else {
            return false;
        };
        if from == to {
            return false;
        }
        let key = self.order.remove(from);
        self.order.insert(to, key);
        true
    }

    /// The + New group button: an empty group named `name` (trimmed), placed after every other
    /// group. Returns its key, or `None` for a blank name.
    pub fn make_group(&mut self, name: &str) -> Option<String> {
        let name = group_name(name)?;
        let key = self.next_group_key();
        self.groups.push(FeedGroup {
            key: key.clone(),
            name,
            jobs: Vec::new(),
        });
        Some(key)
    }

    /// True when the group keyed `key` took the new name.
    pub fn rename_group(&mut self, key: &str, name: &str) -> bool {
        let (Some(group), Some(name)) = (
            self.groups.iter_mut().find(|group| group.key == key),
            group_name(name),
        ) else {
            return false;
        };
        if group.name == name {
            return false;
        }
        group.name = name;
        true
    }

    /// Deletes the group; its jobs go back to the jobs in no group. True when it existed.
    pub fn delete_group(&mut self, key: &str) -> bool {
        let before = self.groups.len();
        self.groups.retain(|group| group.key != key);
        self.order.retain(|item| item != key);
        self.collapsed.retain(|item| item != key);
        self.groups.len() != before
    }

    /// Move to group: the job keyed `job` leaves any group it is in for the group keyed `group`.
    /// True when it moved.
    pub fn move_job(&mut self, job: &str, group: &str) -> bool {
        let Some(target) = self.groups.iter().position(|item| item.key == group) else {
            return false;
        };
        if self.groups[target].jobs.iter().any(|item| item == job) {
            return false;
        }
        self.remove_job(job);
        self.groups[target].jobs.push(job.to_string());
        true
    }

    /// Remove from group. True when the job was in one.
    pub fn remove_job(&mut self, job: &str) -> bool {
        let before = self.group_of(job).is_some();
        for group in &mut self.groups {
            group.jobs.retain(|item| item != job);
        }
        before
    }

    /// The channel's jobs the bot filter lets show.
    pub fn shown_jobs<'a, J: FeedJob>(
        &'a self,
        jobs: &'a [J],
        channel: &'a FeedChannel,
    ) -> impl Iterator<Item = usize> + 'a {
        channel
            .jobs
            .iter()
            .copied()
            .filter(move |&job| self.shows_bot(jobs[job].profile()))
    }

    /// The groups Move to group lists for this job: every group but the one it is in.
    pub fn move_targets<'a>(&'a self, job: &'a str) -> impl Iterator<Item = &'a FeedGroup> + 'a {
        let current = self.group_of(job).map(|group| group.key.as_str());
        self.groups
            .iter()
            .filter(move |group| Some(group.key.as_str()) != current)
    }

    pub fn is_collapsed(&self, group: &str) -> bool {
        self.collapsed.iter().any(|item| item == group)
    }

    pub fn toggle_collapsed(&mut self, group: &str) {
        if self.is_collapsed(group) {
            self.collapsed.retain(|item| item != group);
        } else {
            self.collapsed.push(group.to_string());
        }
    }

    /// The first launch with unread counts: every run so far counts as read. True when it marked
    /// any.
    pub fn start_reading<J: FeedJob>(&mut self, jobs: &[J]) -> bool {
        if !self.read.is_empty() {
            return false;
        }
        for job in jobs {
            self.read_job(job);
        }
        !self.read.is_empty()
    }

    /// Opening the channel keyed `selected` reads every run of its jobs the bot filter shows;
    /// `#all` (`None`) reads nothing. True when a marker moved.
    pub fn read_channel<J: FeedJob>(
        &mut self,
        jobs: &[J],
        channels: &[FeedChannel],
        selected: Option<&str>,
    ) -> bool {
        let Some(channel) = selected.and_then(|key| channels.iter().find(|item| item.key == key))
        else {
            return false;
        };
        let shown: Vec<usize> = self.shown_jobs(jobs, channel).collect();
        let mut moved = false;
        for job in shown {
            moved |= self.read_job(&jobs[job]);
        }
        moved
    }

    /// The job's runs newer than the newest one read.
    pub fn unread<J: FeedJob>(&self, job: &J) -> usize {
        let Some(read) = self.read.get(job.key()) else {
            return job.run_count();
        };
        (0..job.run_count())
            .rev()
            .take_while(|&run| job.run_at(run) > read.as_str())
            .count()
    }

    /// Marks the job's newest run read. True when the marker moved.
    fn read_job<J: FeedJob>(&mut self, job: &J) -> bool {
        let Some(newest) = job.run_count().checked_sub(1).map(|run| job.run_at(run)) else {
            return false;
        };
        if self
            .read
            .get(job.key())
            .is_some_and(|read| read.as_str() >= newest)
        {
            return false;
        }
        self.read.insert(job.key().to_string(), newest.to_string());
        true
    }

    /// The group holding this job. A job saved into two (a hand-edited value) belongs to the first.
    fn group_of(&self, job: &str) -> Option<&FeedGroup> {
        self.groups
            .iter()
            .find(|group| group.jobs.iter().any(|item| item == job))
    }

    /// `channel:<n>` past every number a group or the order still holds.
    fn next_group_key(&self) -> String {
        let number = self
            .groups
            .iter()
            .map(|group| &group.key)
            .chain(&self.order)
            .filter_map(|key| key.strip_prefix(GROUP_KEY_PREFIX)?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        format!("{GROUP_KEY_PREFIX}{}", number + 1)
    }
}

/// A group name trimmed and capped, or `None` when nothing is left.
fn group_name(name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(MAX_GROUP_NAME_CHARS).collect();
    (!name.is_empty()).then_some(name)
}

/// The channels the list can draw: every group, then every job's own channel, each job's in the
/// group it sits in.
pub fn feed_channels<J: FeedJob>(jobs: &[J], prefs: &FeedPrefs) -> Vec<FeedChannel> {
    let homes: Vec<Option<&str>> = jobs
        .iter()
        .map(|job| prefs.group_of(job.key()).map(|group| group.key.as_str()))
        .collect();
    let groups = prefs.groups.iter().map(|group| FeedChannel {
        key: group.key.clone(),
        name: group.name.clone(),
        jobs: (0..jobs.len())
            .filter(|&job| homes[job] == Some(group.key.as_str()))
            .collect(),
        kind: FeedChannelKind::Group,
    });
    let own = jobs.iter().enumerate().map(|(index, job)| FeedChannel {
        key: job.key().to_string(),
        name: job.name().to_string(),
        jobs: vec![index],
        kind: FeedChannelKind::Job {
            group: homes[index].map(str::to_string),
        },
    });
    groups.chain(own).collect()
}

/// The rows of the channel list below `#all`, as indexes into the channels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShownChannels {
    /// The groups, each followed by its jobs' channels unless it is collapsed, then the jobs in no
    /// group.
    pub rows: Vec<usize>,
    /// Where the jobs in no group start in `rows`.
    pub loose: usize,
}

/// The channels the list shows, in the order it shows them. Each section (the groups, one group's
/// jobs, the jobs in no group) is ordered by its newest job or by the manual order; a channel shows
/// when it holds a job of the filtered bot, and an empty group only while every bot is listed.
pub fn shown_channels<J: FeedJob>(
    jobs: &[J],
    channels: &[FeedChannel],
    prefs: &FeedPrefs,
) -> ShownChannels {
    let section = |kind: &FeedChannelKind| {
        let mut shown: Vec<(usize, usize)> = channels
            .iter()
            .enumerate()
            .filter(|(_, channel)| channel.kind == *kind)
            .filter_map(|(index, channel)| {
                match prefs.shown_jobs(jobs, channel).next() {
                    Some(job) => Some((index, job)),
                    // Only a group can hold no job.
                    None if channel.jobs.is_empty() && prefs.bot.is_none() => {
                        Some((index, usize::MAX))
                    }
                    None => None,
                }
            })
            .collect();
        match prefs.sort {
            FeedSort::Activity => shown.sort_by_key(|&(_, newest)| newest),
            FeedSort::Manual => shown.sort_by_cached_key(|&(index, _)| {
                prefs
                    .order
                    .iter()
                    .position(|key| *key == channels[index].key)
                    .unwrap_or(usize::MAX)
            }),
        }
        shown.into_iter().map(|(index, _)| index)
    };
    let mut rows = Vec::new();
    for group in section(&FeedChannelKind::Group) {
        rows.push(group);
        let key = &channels[group].key;
        if !prefs.is_collapsed(key) {
            rows.extend(section(&FeedChannelKind::Job {
                group: Some(key.clone()),
            }));
        }
    }
    let loose = rows.len();
    rows.extend(section(&FeedChannelKind::Job { group: None }));
    ShownChannels { rows, loose }
}

/// The channel a view shows: `selected` while the list shows it, the group of a job's channel
/// hidden by that group's collapse, or else `#all` (`None`), which with `#all` hidden is the first
/// channel the list shows.
pub fn resolve_selection(
    channels: &[FeedChannel],
    shown: &ShownChannels,
    prefs: &FeedPrefs,
    selected: Option<&str>,
) -> Option<String> {
    let is_shown = |key: &str| shown.rows.iter().any(|&index| channels[index].key == key);
    if let Some(key) = selected {
        if is_shown(key) {
            return Some(key.to_string());
        }
        let collapsed_group = channels.iter().find_map(|channel| match &channel.kind {
            FeedChannelKind::Job { group: Some(group) } if channel.key == key => Some(group),
            _ => None,
        });
        if let Some(group) =
            collapsed_group.filter(|group| prefs.is_collapsed(group) && is_shown(group))
        {
            return Some(group.clone());
        }
    }
    if prefs.hide_all {
        shown.rows.first().map(|&index| channels[index].key.clone())
    } else {
        None
    }
}

/// The unread runs of the channel's jobs the bot filter shows.
pub fn channel_unread<J: FeedJob>(jobs: &[J], channel: &FeedChannel, prefs: &FeedPrefs) -> usize {
    prefs
        .shown_jobs(jobs, channel)
        .map(|job| prefs.unread(&jobs[job]))
        .sum()
}

/// `#all` (every job once) or the channel keyed `selected`: the runs of its jobs the bot filter
/// lets show, merged oldest first.
pub fn feed_runs<J: FeedJob>(
    jobs: &[J],
    channels: &[FeedChannel],
    selected: Option<&str>,
    prefs: &FeedPrefs,
) -> Vec<FeedRun> {
    let mut runs: Vec<FeedRun> = (0..channels.len())
        .filter(|&channel| match selected {
            Some(key) => channels[channel].key == key,
            // Every job's own channel, since a job in a group has one too.
            None => matches!(channels[channel].kind, FeedChannelKind::Job { .. }),
        })
        .flat_map(|channel| {
            prefs
                .shown_jobs(jobs, &channels[channel])
                .flat_map(move |job| {
                    (0..jobs[job].run_count()).map(move |run| FeedRun { channel, job, run })
                })
        })
        .collect();
    runs.sort_by(|left, right| {
        jobs[left.job]
            .run_at(left.run)
            .cmp(jobs[right.job].run_at(right.run))
    });
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Job {
        key: String,
        name: String,
        profile: String,
        runs: Vec<String>,
    }

    impl FeedJob for Job {
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
            &self.runs[run]
        }
    }

    fn job(profile: &str, id: &str, name: &str, runs: &[&str]) -> Job {
        Job {
            key: job_key(profile, id),
            name: name.to_string(),
            profile: profile.to_string(),
            runs: runs.iter().map(|run| run.to_string()).collect(),
        }
    }

    /// Four jobs across three bots, newest activity first as gxserver sends them.
    fn jobs() -> Vec<Job> {
        vec![
            job("harry", "watch", "Review watcher", &["2026-09-27 10:00:00"]),
            job(
                "default",
                "brief",
                "Morning briefing",
                &["2026-09-26 08:00:00", "2026-09-27 08:00:00"],
            ),
            job(
                "harry",
                "quota",
                "Quota watchdog",
                &["2026-09-25 09:30:00", "2026-09-27 07:30:00"],
            ),
            job(
                "dobby",
                "backup",
                "Nightly backup",
                &["2026-09-26 04:00:00"],
            ),
        ]
    }

    /// The keys of the rows the list shows, a job's channel under a group marked with `  `.
    fn rows(jobs: &[Job], prefs: &FeedPrefs) -> Vec<String> {
        let channels = feed_channels(jobs, prefs);
        shown_channels(jobs, &channels, prefs)
            .rows
            .into_iter()
            .map(|index| match &channels[index].kind {
                FeedChannelKind::Job { group: Some(_) } => format!("  {}", channels[index].key),
                _ => channels[index].key.clone(),
            })
            .collect()
    }

    fn run_times<'a>(jobs: &'a [Job], prefs: &FeedPrefs, selected: Option<&str>) -> Vec<&'a str> {
        let channels = feed_channels(jobs, prefs);
        feed_runs(jobs, &channels, selected, prefs)
            .iter()
            .map(|run| jobs[run.job].run_at(run.run))
            .collect()
    }

    /// A group named Alerts holding the two harry jobs.
    fn alerts() -> FeedPrefs {
        let mut prefs = FeedPrefs::default();
        let alerts = prefs.make_group("Alerts").expect("group");
        assert!(prefs.move_job("harry/quota", &alerts));
        assert!(prefs.move_job("harry/watch", &alerts));
        prefs
    }

    #[test]
    fn every_job_starts_in_its_own_channel_named_after_it() {
        let jobs = jobs();
        let channels = feed_channels(&jobs, &FeedPrefs::default());
        assert_eq!(channels.len(), 4);
        assert_eq!(channels[1].key, "default/brief");
        assert_eq!(channels[1].name, "Morning briefing");
        assert_eq!(channels[1].jobs, [1]);
        assert_eq!(channels[1].kind, FeedChannelKind::Job { group: None });
        assert_eq!(
            rows(&jobs, &FeedPrefs::default()),
            [
                "harry/watch",
                "default/brief",
                "harry/quota",
                "dobby/backup"
            ]
        );
    }

    #[test]
    fn a_new_group_is_empty_named_and_listed_above_the_jobs() {
        let jobs = jobs();
        let mut prefs = FeedPrefs::default();
        assert_eq!(prefs.make_group("   "), None);
        assert_eq!(prefs, FeedPrefs::default());
        assert_eq!(prefs.make_group("  Alerts ").as_deref(), Some("channel:1"));
        let channels = feed_channels(&jobs, &prefs);
        assert_eq!(channels[0].name, "Alerts");
        assert_eq!(channels[0].kind, FeedChannelKind::Group);
        assert!(channels[0].jobs.is_empty());
        assert_eq!(
            rows(&jobs, &prefs),
            [
                "channel:1",
                "harry/watch",
                "default/brief",
                "harry/quota",
                "dobby/backup"
            ]
        );
    }

    #[test]
    fn a_job_moved_into_a_group_sits_under_it_and_nowhere_else() {
        let jobs = jobs();
        let prefs = alerts();
        let channels = feed_channels(&jobs, &prefs);
        assert_eq!(channels[0].jobs, [0, 2]);
        assert_eq!(shown_channels(&jobs, &channels, &prefs).loose, 3);
        assert_eq!(
            rows(&jobs, &prefs),
            [
                "channel:1",
                "  harry/watch",
                "  harry/quota",
                "default/brief",
                "dobby/backup"
            ]
        );
    }

    #[test]
    fn a_group_shows_its_jobs_runs_merged_by_time_and_a_job_channel_only_its_own() {
        let jobs = jobs();
        let prefs = alerts();
        assert_eq!(
            run_times(&jobs, &prefs, Some("channel:1")),
            [
                "2026-09-25 09:30:00",
                "2026-09-27 07:30:00",
                "2026-09-27 10:00:00"
            ]
        );
        assert_eq!(
            run_times(&jobs, &prefs, Some("harry/quota")),
            ["2026-09-25 09:30:00", "2026-09-27 07:30:00"]
        );
        // #all shows every run once, each under its job's own channel.
        let channels = feed_channels(&jobs, &prefs);
        let all = feed_runs(&jobs, &channels, None, &prefs);
        assert_eq!(all.len(), 6);
        assert!(all
            .iter()
            .all(|run| channels[run.channel].key == jobs[run.job].key));
    }

    #[test]
    fn a_job_sits_in_one_group_at_a_time() {
        let mut prefs = alerts();
        let backups = prefs.make_group("Backups").expect("group");
        assert!(prefs.move_job("harry/quota", &backups));
        assert!(!prefs.move_job("harry/quota", &backups));
        assert!(!prefs.move_job("harry/quota", "channel:9"));
        assert!(!prefs.move_job("harry/quota", "default/brief"));
        assert_eq!(prefs.groups[0].jobs, ["harry/watch"]);
        assert_eq!(prefs.groups[1].jobs, ["harry/quota"]);
        let targets: Vec<&str> = prefs
            .move_targets("harry/quota")
            .map(|group| group.key.as_str())
            .collect();
        assert_eq!(targets, ["channel:1"]);
        assert_eq!(prefs.move_targets("dobby/backup").count(), 2);
    }

    #[test]
    fn an_emptied_group_stays_until_it_is_deleted_and_its_jobs_go_back_below() {
        let jobs = jobs();
        let mut prefs = alerts();
        assert!(prefs.remove_job("harry/watch"));
        assert!(!prefs.remove_job("harry/watch"));
        assert!(prefs.remove_job("harry/quota"));
        assert_eq!(rows(&jobs, &prefs)[0], "channel:1");
        let mut prefs = alerts();
        prefs.toggle_collapsed("channel:1");
        assert!(prefs.delete_group("channel:1"));
        assert!(!prefs.delete_group("channel:1"));
        assert!(prefs.groups.is_empty() && prefs.collapsed.is_empty());
        assert!(!prefs.order.contains(&"channel:1".to_string()));
        assert_eq!(
            rows(&jobs, &prefs),
            [
                "harry/watch",
                "default/brief",
                "harry/quota",
                "dobby/backup"
            ]
        );
    }

    #[test]
    fn only_a_group_can_be_renamed_and_never_to_a_blank_name() {
        let mut prefs = alerts();
        assert!(prefs.rename_group("channel:1", " Pages "));
        assert_eq!(prefs.groups[0].name, "Pages");
        assert!(!prefs.rename_group("channel:1", "Pages"));
        assert!(!prefs.rename_group("channel:1", "  "));
        assert!(!prefs.rename_group("harry/watch", "Watcher"));
        assert_eq!(prefs.groups[0].name, "Pages");
    }

    #[test]
    fn a_collapsed_group_hides_its_jobs_and_stays_collapsed_after_a_restart() {
        let jobs = jobs();
        let mut prefs = alerts();
        prefs.toggle_collapsed("channel:1");
        assert!(prefs.is_collapsed("channel:1"));
        assert_eq!(
            rows(&jobs, &prefs),
            ["channel:1", "default/brief", "dobby/backup"]
        );
        let restored: FeedPrefs =
            serde_json::from_value(serde_json::to_value(&prefs).expect("save")).expect("restore");
        assert_eq!(restored, prefs);
        prefs.toggle_collapsed("channel:1");
        assert!(!prefs.is_collapsed("channel:1"));
    }

    #[test]
    fn the_bot_filter_narrows_groups_to_that_bots_jobs_and_hides_empty_groups() {
        let jobs = jobs();
        let mut prefs = alerts();
        prefs.move_job("default/brief", "channel:1");
        prefs.make_group("Empty");
        prefs.bot = Some("harry".into());
        assert_eq!(
            rows(&jobs, &prefs),
            ["channel:1", "  harry/watch", "  harry/quota"]
        );
        assert_eq!(
            run_times(&jobs, &prefs, Some("channel:1")),
            [
                "2026-09-25 09:30:00",
                "2026-09-27 07:30:00",
                "2026-09-27 10:00:00"
            ]
        );
        prefs.bot = Some("dobby".into());
        assert_eq!(rows(&jobs, &prefs), ["dobby/backup"]);
    }

    #[test]
    fn newest_activity_orders_each_section_by_its_newest_job_and_puts_empty_groups_last() {
        let jobs = jobs();
        let mut prefs = FeedPrefs::default();
        prefs.make_group("Empty");
        let backups = prefs.make_group("Backups").expect("group");
        prefs.move_job("dobby/backup", &backups);
        prefs.move_job("default/brief", &backups);
        assert_eq!(
            rows(&jobs, &prefs),
            [
                "channel:2",
                "  default/brief",
                "  dobby/backup",
                "channel:1",
                "harry/watch",
                "harry/quota"
            ]
        );
    }

    #[test]
    fn manual_order_moves_a_channel_only_within_its_own_section() {
        let jobs = jobs();
        let mut prefs = alerts();
        prefs.make_group("Backups");
        prefs.sort = FeedSort::Manual;
        prefs.place_new_channels(&feed_channels(&jobs, &prefs));
        let channels = feed_channels(&jobs, &prefs);
        assert!(prefs.move_channel("channel:2", "channel:1", &channels));
        assert!(prefs.move_channel("harry/quota", "harry/watch", &channels));
        assert!(prefs.move_channel("dobby/backup", "default/brief", &channels));
        assert!(!prefs.move_channel("default/brief", "harry/watch", &channels));
        assert!(!prefs.move_channel("harry/watch", "channel:2", &channels));
        assert_eq!(
            rows(&jobs, &prefs),
            [
                "channel:2",
                "channel:1",
                "  harry/quota",
                "  harry/watch",
                "dobby/backup",
                "default/brief"
            ]
        );
    }

    /// A later load of the feed: Review watcher and Morning briefing ran once more.
    fn later(jobs: &mut [Job]) {
        jobs[0].runs.push("2026-09-27 12:00:00".into());
        jobs[1].runs.push("2026-09-27 12:30:00".into());
    }

    #[test]
    fn unread_starts_at_zero_on_the_first_launch_then_counts_new_runs() {
        let mut jobs = jobs();
        let mut prefs = alerts();
        assert!(prefs.start_reading(&jobs));
        assert!(!prefs.start_reading(&jobs));
        assert!(jobs.iter().all(|job| prefs.unread(job) == 0));
        later(&mut jobs);
        let channels = feed_channels(&jobs, &prefs);
        assert_eq!(prefs.unread(&jobs[0]), 1);
        assert_eq!(prefs.unread(&jobs[1]), 1);
        assert_eq!(channel_unread(&jobs, &channels[0], &prefs), 1);
    }

    #[test]
    fn a_job_never_read_counts_every_run_unread() {
        let mut jobs = jobs();
        let mut prefs = FeedPrefs::default();
        prefs.start_reading(&jobs);
        jobs.insert(
            0,
            job(
                "harry",
                "fresh",
                "Fresh job",
                &["2026-09-27 11:00:00", "2026-09-27 11:30:00"],
            ),
        );
        assert!(!prefs.start_reading(&jobs));
        assert_eq!(prefs.unread(&jobs[0]), 2);
    }

    #[test]
    fn opening_a_group_or_a_jobs_channel_reads_only_its_jobs_and_all_reads_nothing() {
        let mut jobs = jobs();
        let mut prefs = alerts();
        prefs.start_reading(&jobs);
        later(&mut jobs);
        jobs[2].runs.push("2026-09-27 13:00:00".into());
        let channels = feed_channels(&jobs, &prefs);
        let unread =
            |prefs: &FeedPrefs| jobs.iter().map(|job| prefs.unread(job)).collect::<Vec<_>>();
        assert!(!prefs.read_channel(&jobs, &channels, None));
        assert_eq!(unread(&prefs), [1, 1, 1, 0]);
        assert!(prefs.read_channel(&jobs, &channels, Some("harry/watch")));
        assert_eq!(unread(&prefs), [0, 1, 1, 0]);
        assert!(prefs.read_channel(&jobs, &channels, Some("channel:1")));
        assert_eq!(unread(&prefs), [0, 1, 0, 0]);
        assert!(!prefs.read_channel(&jobs, &channels, Some("channel:1")));
    }

    #[test]
    fn the_bot_filter_narrows_a_groups_unread_count_and_what_opening_it_reads() {
        let mut jobs = jobs();
        let mut prefs = alerts();
        prefs.move_job("default/brief", "channel:1");
        prefs.start_reading(&jobs);
        later(&mut jobs);
        let channels = feed_channels(&jobs, &prefs);
        assert_eq!(channel_unread(&jobs, &channels[0], &prefs), 2);
        prefs.bot = Some("harry".into());
        assert_eq!(channel_unread(&jobs, &channels[0], &prefs), 1);
        prefs.read_channel(&jobs, &channels, Some("channel:1"));
        prefs.bot = None;
        assert_eq!(channel_unread(&jobs, &channels[0], &prefs), 1);
        assert_eq!(prefs.unread(&jobs[1]), 1);
    }

    #[test]
    fn the_selection_falls_to_a_collapsed_group_or_with_all_hidden_to_the_first_channel() {
        let jobs = jobs();
        let resolve = |prefs: &FeedPrefs, selected: Option<&str>| {
            let channels = feed_channels(&jobs, prefs);
            let shown = shown_channels(&jobs, &channels, prefs);
            resolve_selection(&channels, &shown, prefs, selected)
        };
        let mut prefs = alerts();
        assert_eq!(
            resolve(&prefs, Some("harry/watch")).as_deref(),
            Some("harry/watch")
        );
        prefs.toggle_collapsed("channel:1");
        assert_eq!(
            resolve(&prefs, Some("harry/watch")).as_deref(),
            Some("channel:1")
        );
        assert_eq!(resolve(&prefs, Some("channel:9")), None);
        assert_eq!(resolve(&prefs, None), None);
        prefs.hide_all = true;
        assert_eq!(resolve(&prefs, None).as_deref(), Some("channel:1"));
        prefs.bot = Some("dobby".into());
        assert_eq!(
            resolve(&prefs, Some("harry/watch")).as_deref(),
            Some("dobby/backup")
        );
    }

    #[test]
    fn a_groups_shown_jobs_follow_the_bot_filter() {
        let jobs = jobs();
        let mut prefs = alerts();
        prefs.move_job("default/brief", "channel:1");
        let channels = feed_channels(&jobs, &prefs);
        assert_eq!(prefs.shown_jobs(&jobs, &channels[0]).count(), 3);
        prefs.bot = Some("harry".into());
        assert_eq!(
            prefs.shown_jobs(&jobs, &channels[0]).collect::<Vec<_>>(),
            [0, 2]
        );
    }

    #[test]
    fn read_markers_and_a_hidden_all_survive_a_restart() {
        let jobs = jobs();
        let mut prefs = FeedPrefs {
            hide_all: true,
            ..FeedPrefs::default()
        };
        prefs.start_reading(&jobs);
        let saved = serde_json::to_value(&prefs).expect("save");
        assert_eq!(saved["hideAll"], true);
        assert_eq!(saved["read"]["harry/quota"], "2026-09-27 07:30:00");
        let restored: FeedPrefs = serde_json::from_value(saved).expect("restore");
        assert_eq!(restored, prefs);
    }

    #[test]
    fn a_saved_value_from_before_groups_still_reads_as_groups() {
        let old: FeedPrefs = serde_json::from_value(json!({
            "sort": "manual", "order": ["harry/watch", "channel:3"], "bot": "harry"
        }))
        .expect("old value");
        assert_eq!(old.sort, FeedSort::Manual);
        assert!(old.groups.is_empty() && old.collapsed.is_empty());
        assert!(old.read.is_empty() && !old.hide_all);

        let jobs = jobs();
        let mut made: FeedPrefs = serde_json::from_value(json!({
            "order": ["channel:3"],
            "channels": [{"key": "channel:3", "name": "hh", "jobs": ["dobby/backup"]}]
        }))
        .expect("made channel");
        assert_eq!(
            rows(&jobs, &made),
            [
                "channel:3",
                "  dobby/backup",
                "harry/watch",
                "default/brief",
                "harry/quota"
            ]
        );
        assert_eq!(made.make_group("Next").as_deref(), Some("channel:4"));
        let saved = serde_json::to_value(&made).expect("save");
        assert_eq!(saved["channels"][0]["name"], "hh");
    }
}
