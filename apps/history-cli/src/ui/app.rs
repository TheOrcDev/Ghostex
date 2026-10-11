use crate::model::Session;
use crate::scan;
use crossterm::event;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::MouseEvent;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use ratatui::Terminal;
use std::io;
use std::io::Stdout;
use std::time::Duration;

use super::*;

pub(super) const PICKER_CHROME_HEIGHT: u16 = 8;
pub(super) const FOOTER_HEIGHT: u16 = 4;
pub(super) const ROW_GAP_COMFORTABLE: usize = 1;
pub(super) const TRANSCRIPT_HINT_HEIGHT: u16 = 3;
pub(super) const MOUSE_SCROLL_LINES: usize = 3;
pub(super) const LIVE_PREFIX_COLS: usize = 2;
pub(super) const USER_MESSAGE_BG_DARK: (u8, u8, u8) = (30, 30, 30);

pub struct App {
    pub(super) accept_all_resume: bool,
    pub(super) sessions: Vec<Session>,
    pub(super) filtered: Vec<usize>,
    pub(super) query: String,
    pub(super) selected: usize,
    pub(super) scroll_top: usize,
    pub(super) density: Density,
    pub(super) toolbar_focus: ToolbarControl,
    pub(super) filter_mode: FilterMode,
    pub(super) sort_key: SortKey,
    pub(super) current_cwd: String,
    pub(super) expanded: Option<usize>,
    pub(super) screen: Screen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Density {
    Comfortable,
    Dense,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ToolbarControl {
    Filter,
    Sort,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FilterMode {
    All,
    Cwd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SortKey {
    Updated,
    Created,
}

pub(super) enum Screen {
    List,
    Transcript {
        session_index: usize,
        scroll: usize,
        highlight_block: Option<usize>,
    },
}

pub(super) enum AppAction {
    Continue,
    Quit,
    Resume(usize),
}

pub fn run(sessions: Vec<Session>, accept_all_resume: bool) -> io::Result<()> {
    let mut terminal = enter_terminal()?;
    let current_cwd = std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut app = App::new(sessions, current_cwd, accept_all_resume);
    let action_result = app.run(&mut terminal);
    leave_terminal(&mut terminal)?;
    match action_result? {
        AppAction::Continue | AppAction::Quit => Ok(()),
        AppAction::Resume(session_index) => app.resume_session(session_index),
    }
}

impl App {
    fn new(sessions: Vec<Session>, current_cwd: String, accept_all_resume: bool) -> Self {
        let mut app = Self {
            accept_all_resume,
            sessions,
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            scroll_top: 0,
            density: Density::Comfortable,
            toolbar_focus: ToolbarControl::Filter,
            filter_mode: FilterMode::All,
            sort_key: SortKey::Updated,
            current_cwd,
            expanded: None,
            screen: Screen::List,
        };
        app.apply_filter_and_sort();
        app
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<AppAction> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            if !event::poll(Duration::from_millis(250))? {
                continue;
            }
            let action = match event::read()? {
                Event::Key(key) => {
                    if matches!(key.kind, KeyEventKind::Release) {
                        AppAction::Continue
                    } else {
                        self.handle_key(key)
                    }
                }
                Event::Mouse(mouse) => self.handle_mouse(mouse),
                _ => AppAction::Continue,
            };
            match action {
                AppAction::Continue => {}
                action => return Ok(action),
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        match self.screen {
            Screen::List => self.draw_list(frame),
            Screen::Transcript {
                session_index,
                scroll,
                highlight_block,
            } => self.draw_transcript(frame, session_index, scroll, highlight_block),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        match self.screen {
            Screen::List => self.handle_list_key(key),
            Screen::Transcript { .. } => self.handle_transcript_key(key),
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> AppAction {
        match self.screen {
            Screen::List => AppAction::Continue,
            Screen::Transcript { .. } => self.handle_transcript_mouse(mouse),
        }
    }
}

impl App {
    pub(super) fn open_selected_transcript(&mut self) {
        let Some(session_index) = self.filtered.get(self.selected).copied() else {
            return;
        };
        if !self.sessions[session_index].transcript_loaded {
            if let Ok(blocks) = scan::load_transcript(&self.sessions[session_index]) {
                self.sessions[session_index].blocks = blocks;
                self.sessions[session_index].transcript_loaded = true;
            }
        }
        self.screen = Screen::Transcript {
            session_index,
            scroll: usize::MAX,
            highlight_block: None,
        };
    }

    pub(super) fn resume_selected_action(&self) -> AppAction {
        self.filtered
            .get(self.selected)
            .copied()
            .map(AppAction::Resume)
            .unwrap_or(AppAction::Continue)
    }

    fn resume_session(&self, session_index: usize) -> io::Result<()> {
        let Some(session) = self.sessions.get(session_index) else {
            return Ok(());
        };
        resume_agent_session(session, self.accept_all_resume)
    }

    pub(super) fn toggle_expanded(&mut self) {
        let Some(session_index) = self.filtered.get(self.selected).copied() else {
            return;
        };
        self.expanded = if self.expanded == Some(session_index) {
            None
        } else {
            Some(session_index)
        };
    }

    pub(super) fn toggle_density(&mut self) {
        self.density = match self.density {
            Density::Comfortable => Density::Dense,
            Density::Dense => Density::Comfortable,
        };
    }

    pub(super) fn change_toolbar_value(&mut self) {
        match self.toolbar_focus {
            ToolbarControl::Filter => {
                self.filter_mode = match self.filter_mode {
                    FilterMode::All => FilterMode::Cwd,
                    FilterMode::Cwd => FilterMode::All,
                };
                self.apply_filter_and_sort();
            }
            ToolbarControl::Sort => {
                self.sort_key = match self.sort_key {
                    SortKey::Updated => SortKey::Created,
                    SortKey::Created => SortKey::Updated,
                };
                self.apply_filter_and_sort();
            }
        }
    }

    pub(super) fn apply_filter_and_sort(&mut self) {
        self.filtered = self
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| self.filter_session(session))
            .map(|(index, _)| index)
            .collect();
        let sessions = &self.sessions;
        let sort_key = self.sort_key;
        self.filtered.sort_by(|a, b| {
            let left = &sessions[*a];
            let right = &sessions[*b];
            match sort_key {
                SortKey::Updated => right.updated_at.cmp(&left.updated_at),
                SortKey::Created => right
                    .created_at
                    .unwrap_or(right.updated_at)
                    .cmp(&left.created_at.unwrap_or(left.updated_at)),
            }
            .then_with(|| left.agent.label().cmp(right.agent.label()))
            .then_with(|| left.display_title().cmp(right.display_title()))
        });
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
        self.scroll_top = self.scroll_top.min(self.filtered.len().saturating_sub(1));
    }

    fn filter_session(&self, session: &Session) -> bool {
        if self.filter_mode == FilterMode::Cwd
            && (self.current_cwd.is_empty() || session.project != self.current_cwd)
        {
            return false;
        }
        self.query.is_empty() || session.matches_query(&self.query)
    }

    pub(super) fn move_selection(&mut self, direction: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let max = self.filtered.len().saturating_sub(1);
        self.selected = if direction.is_negative() {
            self.selected.saturating_sub(direction.unsigned_abs())
        } else {
            self.selected.saturating_add(direction as usize).min(max)
        };
    }

    pub(super) fn page_selection(&mut self, direction: isize) {
        let step = 10;
        self.move_selection(direction * step);
    }

    pub(super) fn jump_selection_top(&mut self) {
        self.selected = 0;
    }

    pub(super) fn jump_selection_bottom(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
        }
    }

    pub(super) fn ensure_selected_visible(&mut self, height: usize) {
        if self.filtered.is_empty() {
            self.scroll_top = 0;
            return;
        }
        let height = height.max(1);
        if self.selected < self.scroll_top {
            self.scroll_top = self.selected;
        } else if self.selected >= self.scroll_top + height {
            self.scroll_top = self.selected.saturating_sub(height - 1);
        }
    }

    pub(super) fn has_more_below(&self, height: usize) -> bool {
        self.scroll_top + height < self.filtered.len()
    }
}

fn enter_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn leave_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()
}
