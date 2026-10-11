use crate::agent::ALL_AGENTS;
use crate::index::day_key;
use crate::scan::project_display_name;
use crate::unicode as uni;

use super::*;

impl<'a> Tui<'a> {
    // -----------------------------------------------------------------------
    // state transitions
    // -----------------------------------------------------------------------

    /// Toggle the favorite flag of the selected prompt, persist, and re-rank.
    /// Selection follows the same record across the re-sort so the cursor does
    /// not jump after starring/unstarring.
    pub(super) fn toggle_favorite(&mut self) {
        let Some(hit) = self.hits.get(self.sel) else {
            return;
        };
        let rec_idx = hit.index;
        let rec = &self.index.records[rec_idx];
        let (agent, text) = (rec.agent, rec.text.clone());
        self.index.toggle_favorite(agent, &text);
        self.recompute();
        if let Some(j) = self.hits.iter().position(|h| h.index == rec_idx) {
            self.sel = j;
        }
    }

    pub(super) fn toggle_day_grouping(&mut self) {
        self.group_by_day = !self.group_by_day;
        self.recompute();
    }

    fn select_hit(&mut self, hit_idx: usize) {
        if hit_idx >= self.hits.len() || self.sel == hit_idx {
            return;
        }
        self.sel = hit_idx;
        self.preview_scroll = 0;
        self.result_scroll = 0;
    }

    pub(super) fn jump_day(&mut self, delta: isize) {
        if self.hits.is_empty() || self.sel >= self.hits.len() {
            return;
        }
        let current_day = day_key(self.index.records[self.hits[self.sel].index].ts);
        if delta > 0 {
            for i in (self.sel + 1)..self.hits.len() {
                if day_key(self.index.records[self.hits[i].index].ts) != current_day {
                    self.select_hit(i);
                    return;
                }
            }
            return;
        }
        let mut i = self.sel;
        while i > 0 {
            i -= 1;
            let candidate_day = day_key(self.index.records[self.hits[i].index].ts);
            if candidate_day == current_day {
                continue;
            }
            while i > 0 && day_key(self.index.records[self.hits[i - 1].index].ts) == candidate_day {
                i -= 1;
            }
            self.select_hit(i);
            return;
        }
    }

    pub(super) fn open_agent_filter_picker(&mut self) {
        self.filtering_agent = true;
        self.filter_sel = 0;
    }

    pub(super) fn move_filter_selection(&mut self, delta: isize) {
        if delta < 0 {
            self.filter_sel = if self.filter_sel == 0 {
                ALL_AGENTS.len() - 1
            } else {
                self.filter_sel - 1
            };
        } else {
            self.filter_sel = (self.filter_sel + 1) % ALL_AGENTS.len();
        }
    }

    pub(super) fn toggle_filter_selection(&mut self) {
        self.agent_filter_mask ^= ALL_AGENTS[self.filter_sel].bit();
        self.recompute();
    }

    fn project_matches(&self, project: &str) -> bool {
        if self.project_query.is_empty() {
            return true;
        }
        let q = String::from_utf8_lossy(&self.project_query).to_lowercase();
        let base = project_display_name(project).to_lowercase();
        base.contains(&q) || project.to_lowercase().contains(&q)
    }

    pub(super) fn filtered_projects(&self) -> Vec<&str> {
        self.index
            .projects()
            .into_iter()
            .filter(|p| self.project_matches(p))
            .collect()
    }

    pub(super) fn open_project_filter_picker(&mut self) {
        self.filtering_project = true;
        self.project_sel = 0;
        self.project_query.clear();
    }

    pub(super) fn move_project_selection(&mut self, delta: isize) {
        let count = self.filtered_projects().len();
        if count == 0 {
            return;
        }
        if delta < 0 {
            self.project_sel = if self.project_sel == 0 {
                count - 1
            } else {
                self.project_sel - 1
            };
        } else {
            self.project_sel = (self.project_sel + 1) % count;
        }
    }

    pub(super) fn apply_project_filter_selection(&mut self) {
        self.project_filter = self
            .filtered_projects()
            .get(self.project_sel)
            .map(|p| p.to_string());
        self.recompute();
    }

    pub(super) fn toggle_project_filter_selection(&mut self) {
        let picked = self
            .filtered_projects()
            .get(self.project_sel)
            .map(|p| p.to_string());
        self.project_filter = match (picked, self.project_filter.clone()) {
            (None, _) => None,
            (Some(p), Some(cur)) if cur == p => None,
            (Some(p), _) => Some(p),
        };
        self.recompute();
    }

    pub(super) fn insert_project_query_byte(&mut self, c: u8) {
        self.project_query.push(c);
        self.project_sel = 0;
    }

    pub(super) fn backspace_project_query(&mut self) {
        if self.project_query.is_empty() {
            return;
        }
        self.project_query.pop();
        while !self.project_query.is_empty()
            && (self.project_query[self.project_query.len() - 1] & 0xC0) == 0x80
        {
            self.project_query.pop();
        }
        self.project_sel = self
            .project_sel
            .min(self.filtered_projects().len().saturating_sub(1));
    }

    pub(super) fn insert_query_byte(&mut self, c: u8) {
        self.query.insert(self.query_cursor, c);
        self.query_cursor += 1;
        self.recompute();
    }

    fn delete_range(&mut self, start: usize, end: usize) {
        if end <= start {
            return;
        }
        self.query.drain(start..end);
        self.query_cursor = start;
        self.recompute();
    }

    pub(super) fn backspace(&mut self) {
        if self.query_cursor == 0 {
            return;
        }
        self.delete_range(
            uni::prev_char(&self.query, self.query_cursor),
            self.query_cursor,
        );
    }

    pub(super) fn kill_to_end(&mut self) {
        self.delete_range(self.query_cursor, self.query.len());
    }

    pub(super) fn kill_to_beginning(&mut self) {
        self.delete_range(0, self.query_cursor);
    }

    pub(super) fn move_left(&mut self) {
        self.query_cursor = uni::prev_char(&self.query, self.query_cursor);
    }

    pub(super) fn move_right(&mut self) {
        self.query_cursor = uni::next_char(&self.query, self.query_cursor);
    }

    pub(super) fn move_word_left(&mut self) {
        let mut p = self.query_cursor;
        while p > 0 && !is_word_byte(self.query[uni::prev_char(&self.query, p)]) {
            p = uni::prev_char(&self.query, p);
        }
        while p > 0 && is_word_byte(self.query[uni::prev_char(&self.query, p)]) {
            p = uni::prev_char(&self.query, p);
        }
        self.query_cursor = p;
    }

    pub(super) fn move_word_right(&mut self) {
        let mut p = self.query_cursor;
        while p < self.query.len() && !is_word_byte(self.query[p]) {
            p = uni::next_char(&self.query, p);
        }
        while p < self.query.len() && is_word_byte(self.query[p]) {
            p = uni::next_char(&self.query, p);
        }
        self.query_cursor = p;
    }

    pub(super) fn delete_word_forward(&mut self) {
        let start = self.query_cursor;
        self.move_word_right();
        self.delete_range(start, self.query_cursor);
    }

    pub(super) fn delete_word_backward(&mut self) {
        let end = self.query_cursor;
        self.move_word_left();
        self.delete_range(self.query_cursor, end);
    }

    pub(super) fn scroll_result(&mut self, delta: isize) {
        if delta < 0 {
            self.result_scroll = self.result_scroll.saturating_sub((-delta) as usize);
        } else {
            self.result_scroll += delta as usize;
        }
    }

    pub(super) fn scroll_result_to_end(&mut self) {
        if let Some(rec) = self.selected_record() {
            self.result_scroll = rec.text.len();
        }
    }

    pub(super) fn scroll_preview(&mut self, delta: isize) {
        if delta < 0 {
            self.preview_scroll = self.preview_scroll.saturating_sub((-delta) as usize);
        } else {
            self.preview_scroll += delta as usize;
        }
    }

    pub(super) fn move_down(&mut self) {
        if self.hits.is_empty() {
            return;
        }
        if self.sel + 1 < self.hits.len() {
            self.sel += 1;
            self.preview_scroll = 0;
            self.result_scroll = 0;
        }
    }

    pub(super) fn move_up(&mut self) {
        if self.sel > 0 {
            self.sel -= 1;
            self.preview_scroll = 0;
            self.result_scroll = 0;
        }
    }
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
