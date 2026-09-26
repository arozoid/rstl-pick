use ratatui::widgets::ListState;

use crate::config::MenuEntry;

/// Top-level application state.
pub struct App {
    pub list: ListState,
    /// The free-text query filtering the entries.
    pub query: String,
    /// True while the search box is "focused" (i.e. typeable). Kept for future
    /// UI polish; typing always edits the query in this simple picker.
    pub quit: bool,
    /// Transient footer message (e.g. "icon copied"), cleared on the next key.
    pub notice: Option<String>,
    /// Every row of the configured menu, in config order.
    pub entries: Vec<MenuEntry>,
    /// Whether each entry's tools are installed (parallel to `entries`).
    available: Vec<bool>,
    /// How many rows the picker shows at once before the list scrolls.
    pub display: usize,
}

impl App {
    pub fn new(entries: Vec<MenuEntry>, display: usize) -> Self {
        let available = entries.iter().map(|e| e.available()).collect();
        let mut list = ListState::default();
        list.select(Some(0));
        Self {
            list,
            query: String::new(),
            quit: false,
            notice: None,
            entries,
            available,
            display,
        }
    }

    /// Show a transient status message.
    pub fn flash(&mut self, msg: impl Into<String>) {
        self.notice = Some(msg.into());
    }

    /// Hide the transient status message.
    pub fn clear_notice(&mut self) {
        self.notice = None;
    }

    /// The rows that match the current search query and are installed.
    pub fn visible(&self) -> Vec<MenuEntry> {
        let q = self.query.trim().to_lowercase();
        self.entries
            .iter()
            .zip(&self.available)
            .filter(|(_, avail)| **avail)
            .filter(|(entry, _)| q.is_empty() || entry.label.to_lowercase().contains(&q))
            .map(|(entry, _)| entry.clone())
            .collect()
    }

    /// The currently highlighted entry (if the filtered list is non-empty).
    pub fn current_selection(&self) -> Option<MenuEntry> {
        let visible = self.visible();
        let idx = self.list.selected().unwrap_or(0);
        visible.get(idx).cloned()
    }

    /// The installed row bound to `key`, if any.
    pub fn entry_by_key(&self, key: char) -> Option<MenuEntry> {
        self.entries
            .iter()
            .zip(&self.available)
            .find(|(entry, avail)| **avail && entry.key == key)
            .map(|(entry, _)| entry.clone())
    }

    /// Append a character to the search query and reselect the first hit.
    pub fn type_char(&mut self, c: char) {
        if !c.is_control() {
            self.query.push(c);
            self.jump_top();
        }
    }

    /// Remove the last character of the search query.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.jump_top();
    }

    /// Reset the selection to the top of the (re-filtered) list.
    fn jump_top(&mut self) {
        self.list.select(if self.visible().is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// Move the selection up within the filtered list (wraps around).
    pub fn previous(&mut self) {
        let n = self.visible().len();
        if n == 0 {
            return;
        }
        let i = match self.list.selected() {
            Some(i) => (i + n - 1) % n,
            None => 0,
        };
        self.list.select(Some(i));
    }

    /// Move the selection down within the filtered list (wraps around).
    pub fn next(&mut self) {
        let n = self.visible().len();
        if n == 0 {
            return;
        }
        let i = match self.list.selected() {
            Some(i) => (i + 1) % n,
            None => 0,
        };
        self.list.select(Some(i));
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(Vec::new(), 9)
    }
}
