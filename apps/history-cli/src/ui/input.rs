use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use crossterm::terminal;

use super::*;

impl App {
    pub(super) fn handle_list_key(&mut self, key: KeyEvent) -> AppAction {
        if is_ctrl_char(key, 'c') {
            return AppAction::Quit;
        }
        if is_ctrl_char(key, 'r') {
            return self.resume_selected_action();
        }
        if is_ctrl_char(key, 't') || key.code == KeyCode::Enter {
            self.open_selected_transcript();
            return AppAction::Continue;
        }
        if is_ctrl_char(key, 'e') {
            self.toggle_expanded();
            return AppAction::Continue;
        }
        if is_ctrl_char(key, 'o') {
            self.toggle_density();
            return AppAction::Continue;
        }
        match key.code {
            KeyCode::Esc => {
                if self.query.is_empty() {
                    return AppAction::Quit;
                }
                self.query.clear();
                self.apply_filter_and_sort();
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.apply_filter_and_sort();
            }
            KeyCode::Tab => self.toolbar_focus = next_toolbar(self.toolbar_focus),
            KeyCode::BackTab => self.toolbar_focus = previous_toolbar(self.toolbar_focus),
            KeyCode::Left | KeyCode::Right => self.change_toolbar_value(),
            KeyCode::Up if list_plain_nav_allowed(key) => self.move_selection(-1),
            KeyCode::Down if list_plain_nav_allowed(key) => self.move_selection(1),
            KeyCode::PageUp => self.page_selection(-1),
            KeyCode::PageDown => self.page_selection(1),
            KeyCode::Home => self.jump_selection_top(),
            KeyCode::End => self.jump_selection_bottom(),
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_selection(-1)
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_selection(1)
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_selection(-1)
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_selection(1)
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_selection(-1)
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_selection(1)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.query.push(c);
                self.apply_filter_and_sort();
            }
            _ => {}
        }
        AppAction::Continue
    }

    pub(super) fn handle_transcript_key(&mut self, key: KeyEvent) -> AppAction {
        let Screen::Transcript {
            session_index,
            mut scroll,
            mut highlight_block,
        } = self.screen
        else {
            return AppAction::Continue;
        };
        let (terminal_width, area_height) = terminal::size()
            .map(|(width, height)| (width, transcript_content_height(height).max(1) as usize))
            .unwrap_or((80, 10));
        if is_ctrl_char(key, 'r') {
            return AppAction::Resume(session_index);
        }
        if is_ctrl_char(key, 'c') || is_ctrl_char(key, 't') || key.code == KeyCode::Char('q') {
            self.screen = Screen::List;
            return AppAction::Continue;
        }
        let layout = transcript_layout(
            &self.sessions[session_index].blocks,
            terminal_width,
            highlight_block,
        );
        let max_scroll = layout.lines.len().saturating_sub(area_height);
        scroll = if scroll == usize::MAX {
            max_scroll
        } else {
            scroll.min(max_scroll)
        };
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => scroll = scroll.saturating_add(1),
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                scroll = scroll.saturating_sub(area_height)
            }
            KeyCode::PageUp => scroll = scroll.saturating_sub(area_height),
            KeyCode::PageDown | KeyCode::Char(' ') => scroll = scroll.saturating_add(area_height),
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scroll = scroll.saturating_sub(area_height)
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scroll = scroll.saturating_add(area_height)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scroll = scroll.saturating_sub(area_height.saturating_add(1) / 2)
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scroll = scroll.saturating_add(area_height.saturating_add(1) / 2)
            }
            KeyCode::Home => scroll = 0,
            KeyCode::End => scroll = usize::MAX,
            KeyCode::Esc | KeyCode::Left => {
                highlight_block = previous_user_block(&layout.user_blocks, highlight_block);
                if let Some(block_index) = highlight_block {
                    scroll = ensure_block_visible(scroll, &layout, block_index, area_height);
                } else if key.code == KeyCode::Esc {
                    self.screen = Screen::List;
                    return AppAction::Continue;
                }
            }
            KeyCode::Right => {
                highlight_block = next_user_block(&layout.user_blocks, highlight_block);
                if let Some(block_index) = highlight_block {
                    scroll = ensure_block_visible(scroll, &layout, block_index, area_height);
                }
            }
            KeyCode::Enter if highlight_block.is_some() => {
                /*
                 * CDXC:PromptSearch 2026-06-25-21:52:
                 * Codex uses Enter from a selected transcript prompt to edit the live composer.
                 * ghostex-history is a read-only cross-agent browser, so consume the key to preserve the Codex pager contract without mutating or resuming archived sessions.
                 */
            }
            _ => {}
        }
        self.screen = Screen::Transcript {
            session_index,
            scroll,
            highlight_block,
        };
        AppAction::Continue
    }

    pub(super) fn handle_transcript_mouse(&mut self, mouse: MouseEvent) -> AppAction {
        if !matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            return AppAction::Continue;
        }
        let Screen::Transcript {
            session_index,
            mut scroll,
            highlight_block,
        } = self.screen
        else {
            return AppAction::Continue;
        };
        let (terminal_width, area_height) = terminal::size()
            .map(|(width, height)| (width, transcript_content_height(height).max(1) as usize))
            .unwrap_or((80, 10));
        let layout = transcript_layout(
            &self.sessions[session_index].blocks,
            terminal_width,
            highlight_block,
        );
        let max_scroll = layout.lines.len().saturating_sub(area_height);
        scroll = if scroll == usize::MAX {
            max_scroll
        } else {
            scroll.min(max_scroll)
        };
        scroll = match mouse.kind {
            MouseEventKind::ScrollUp => {
                apply_scroll_delta(scroll, -(MOUSE_SCROLL_LINES as isize), max_scroll)
            }
            MouseEventKind::ScrollDown => {
                apply_scroll_delta(scroll, MOUSE_SCROLL_LINES as isize, max_scroll)
            }
            _ => scroll,
        };
        self.screen = Screen::Transcript {
            session_index,
            scroll,
            highlight_block,
        };
        AppAction::Continue
    }
}

fn is_ctrl_char(key: KeyEvent, c: char) -> bool {
    matches!(key.code, KeyCode::Char(ch) if ch == c)
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn list_plain_nav_allowed(key: KeyEvent) -> bool {
    !matches!(key.code, KeyCode::Char(_))
}

fn next_toolbar(control: ToolbarControl) -> ToolbarControl {
    match control {
        ToolbarControl::Filter => ToolbarControl::Sort,
        ToolbarControl::Sort => ToolbarControl::Filter,
    }
}

fn previous_toolbar(control: ToolbarControl) -> ToolbarControl {
    next_toolbar(control)
}
