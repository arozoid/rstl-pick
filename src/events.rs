use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Events the picker loop cares about.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Quit,
    NavigatePrev,
    NavigateNext,
    Type(char),
    Backspace,
    Select,
    /// A key bound to a menu entry was pressed.
    Shortcut(char),
    Redraw,
}

/// Block forever until a relevant event arrives, then classify it.
///
/// `shortcuts` are the keys bound by the menu; pressing one launches its entry
/// instead of adding to the search query.
pub fn next_event(shortcuts: &[char]) -> std::io::Result<AppEvent> {
    loop {
        // A short poll keeps the loop responsive to terminal resizes.
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Resize(_, _) => return Ok(AppEvent::Redraw),
                // Ignore key release/repeat to avoid double-triggering.
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(ev) = classify(&key, shortcuts) {
                        return Ok(ev);
                    }
                }
                _ => {}
            }
        }
    }
}

fn classify(key: &KeyEvent, shortcuts: &[char]) -> Option<AppEvent> {
    // Ctrl+C / Ctrl+\ / Ctrl+Z are ignored entirely (raw mode already disables
    // ISIG; this is belt and braces so no control chord can quit by accident).
    // Any other Ctrl chord is ignored too, so Ctrl+A etc. never pollute.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(AppEvent::Quit),
        KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => Some(AppEvent::NavigatePrev),
        KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => Some(AppEvent::NavigateNext),
        KeyCode::Enter => Some(AppEvent::Select),
        KeyCode::Backspace => Some(AppEvent::Backspace),
        KeyCode::Char(c) if shortcuts.contains(&c) => Some(AppEvent::Shortcut(c)),
        KeyCode::Char(c) => Some(AppEvent::Type(c)),
        _ => None,
    }
}
