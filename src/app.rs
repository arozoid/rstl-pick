use ratatui::widgets::ListState;

/// The terminal apps the picker can launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerItem {
    Network,
    Audio,
    Clipboard,
    Bluetooth,
    Icons,
}

impl PickerItem {
    /// The fixed, ordered list of entries.
    pub const ALL: [PickerItem; 5] = [
        PickerItem::Network,
        PickerItem::Audio,
        PickerItem::Clipboard,
        PickerItem::Bluetooth,
        PickerItem::Icons,
    ];

    /// Human-readable label for the entry.
    pub fn label(self) -> &'static str {
        match self {
            PickerItem::Network => "network",
            PickerItem::Audio => "audio",
            PickerItem::Clipboard => "clipboard",
            PickerItem::Bluetooth => "bluetooth",
            PickerItem::Icons => "icons",
        }
    }

    /// The command it launches in the user's terminal.
    pub fn run(self) -> &'static str {
        match self {
            PickerItem::Network => "nmtui",
            PickerItem::Audio => "wiremix",
            PickerItem::Clipboard => "clipse",
            PickerItem::Bluetooth => "bluetuith",
            PickerItem::Icons => "latuicon",
        }
    }

    /// Shortcut key, shown on the same line as the label.
    pub fn key(self) -> char {
        match self {
            PickerItem::Network => 'n',
            PickerItem::Audio => 'a',
            PickerItem::Clipboard => 'c',
            PickerItem::Bluetooth => 'b',
            PickerItem::Icons => 'i',
        }
    }
}

/// Top-level application state.
pub struct App {
    pub list: ListState,
    /// The free-text query filtering the entries.
    pub query: String,
    /// True while the search box is "focused" (i.e. typeable). Kept for future
    /// UI polish; typing always edits the query in this simple picker.
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut list = ListState::default();
        list.select(Some(0));
        Self {
            list,
            query: String::new(),
            quit: false,
        }
    }

    /// The entries that match the current search query.
    pub fn visible(&self) -> Vec<PickerItem> {
        let q = self.query.trim().to_lowercase();
        PickerItem::ALL
            .iter()
            .copied()
            .filter(|it| q.is_empty() || it.label().contains(&q))
            .collect()
    }

    /// The currently highlighted entry (if the filtered list is non-empty).
    pub fn current_selection(&self) -> Option<PickerItem> {
        let visible = self.visible();
        let idx = self.list.selected().unwrap_or(0);
        visible.get(idx).copied()
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
        Self::new()
    }
}