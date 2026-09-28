use crate::config::MenuEntry;

/// Top-level application state.
pub struct App {
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
    /// Index into the query-matched rows of the highlighted row (see
    /// [`App::matched`]), or `None` when no row can be highlighted.
    selected: Option<usize>,
    /// Index of the first query-matched row the list is scrolled to.
    offset: usize,
}

impl App {
    pub fn new(entries: Vec<MenuEntry>, display: usize) -> Self {
        let available = entries.iter().map(|e| e.available()).collect();
        let mut app = Self {
            query: String::new(),
            quit: false,
            notice: None,
            entries,
            available,
            display,
            selected: None,
            offset: 0,
        };
        app.jump_top();
        app
    }

    /// Show a transient status message.
    pub fn flash(&mut self, msg: impl Into<String>) {
        self.notice = Some(msg.into());
    }

    /// Hide the transient status message.
    pub fn clear_notice(&mut self) {
        self.notice = None;
    }

    /// Indices into `entries` of the rows to show, in config order: the ones
    /// matching the current search query **and** whose program is installed.
    ///
    /// Availability is part of the match, not a rendering decision. A row
    /// whose tools are missing is not a blank line, it is simply not a row:
    /// it leaves no gap, its key does nothing, and the entry below it moves up
    /// instead of being pushed off the bottom of the panel.
    fn matched(&self) -> Vec<usize> {
        let q = self.query.trim().to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| q.is_empty() || entry.label.to_lowercase().contains(&q))
            .filter(|(i, _)| self.available[*i])
            .map(|(i, _)| i)
            .collect()
    }

    /// The rows currently on screen: up to `display` of [`App::matched`],
    /// starting at the scroll offset. Never contains a gap.
    pub fn window(&self) -> Vec<MenuEntry> {
        let matched = self.matched();
        let start = self.offset.min(matched.len());
        matched[start..]
            .iter()
            .take(self.display)
            .map(|&i| self.entries[i].clone())
            .collect()
    }

    /// Position of the highlighted row within [`App::window`], if the
    /// highlighted row is on screen.
    pub fn highlight(&self) -> Option<usize> {
        let sel = self.selected?;
        (self.offset <= sel && sel < self.offset + self.display).then_some(sel - self.offset)
    }

    /// The currently highlighted entry (if any row is visible).
    pub fn current_selection(&self) -> Option<MenuEntry> {
        let i = *self.matched().get(self.selected?)?;
        Some(self.entries[i].clone())
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

    /// Reset the selection to the first visible row, scrolled top.
    fn jump_top(&mut self) {
        self.offset = 0;
        self.selected = if self.matched().is_empty() { None } else { Some(0) };
    }

    /// Move the selection by `dir` visible rows, wrapping around, and scroll
    /// the window to keep it on screen.
    fn step(&mut self, dir: isize) {
        let matched = self.matched();
        let n = matched.len() as isize;
        if n == 0 {
            self.selected = None;
            self.offset = 0;
            return;
        }
        // From no selection, step forward starts at the first row and backward
        // at the last, so both wrap into the list rather than sticking.
        let from = self
            .selected
            .map_or_else(|| if dir > 0 { 0 } else { n - 1 }, |i| i as isize);
        let i = (from + dir).rem_euclid(n) as usize;
        self.selected = Some(i);
        self.scroll_into_view(i);
    }

    /// Scroll the window the minimum amount needed to show matched row `i`.
    fn scroll_into_view(&mut self, i: usize) {
        let last = self.matched().len().saturating_sub(self.display);
        if i < self.offset {
            self.offset = i;
        } else if i >= self.offset + self.display {
            self.offset = i + 1 - self.display;
        }
        self.offset = self.offset.min(last);
    }

    /// Move the selection up within the filtered list (wraps around).
    pub fn previous(&mut self) {
        self.step(-1);
    }

    /// Move the selection down within the filtered list (wraps around).
    pub fn next(&mut self) {
        self.step(1);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(Vec::new(), 9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tool name that is certainly not installed, so `needs` evaluates false.
    const MISSING: &str = "rstl-pick-test-missing";
    /// `sh` is always on PATH, so `needs` evaluates true.
    const PRESENT: &str = "sh";

    fn entry(label: &str, installed: bool) -> MenuEntry {
        MenuEntry {
            label: label.to_string(),
            key: 'a',
            command: "true".to_string(),
            hint: String::new(),
            clip_stdout: false,
            needs: Some(vec![if installed { PRESENT } else { MISSING }.to_string()]),
        }
    }

    /// `n` entries named e01..e=en, where the 1-based indices in `missing`
    /// are not installed.
    fn app(n: usize, display: usize, missing: &[usize]) -> App {
        let entries = (1..=n)
            .map(|i| entry(&format!("e{i:02}"), !missing.contains(&i)))
            .collect();
        App::new(entries, display)
    }

    /// The labels currently in the window, in order.
    fn window_labels(app: &App) -> Vec<String> {
        app.window().iter().map(|e| e.label.clone()).collect()
    }

    #[test]
    fn a_missing_row_takes_no_space() {
        // display 9 of 11, e03/e05/e07 not installed: they are not drawn, so
        // the rows below them move up and the window is filled with the eight
        // entries that are really there.
        let app = app(11, 9, &[3, 5, 7]);
        assert_eq!(
            window_labels(&app),
            vec!["e01", "e02", "e04", "e06", "e08", "e09", "e10", "e11"]
        );
    }

    #[test]
    fn missing_rows_at_the_top_do_not_push_the_first_row_down() {
        // The case that started this: a missing entry above the others used to
        // leave a hole and push everything down by one.
        let app = app(5, 9, &[1, 2]);
        assert_eq!(window_labels(&app), vec!["e03", "e04", "e05"]);
        assert_eq!(app.highlight(), Some(0));
        assert_eq!(app.current_selection().unwrap().label, "e03");
    }

    #[test]
    fn all_installed_shows_a_full_window() {
        let app = app(20, 9, &[]);
        assert_eq!(
            window_labels(&app),
            (1..=9)
                .map(|i| format!("e{i:02}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn budget_is_never_exceeded() {
        for missing in [vec![], vec![1], vec![3, 5, 7], vec![1, 2, 3, 4, 5]] {
            for n in 1..=20 {
                let app = app(n, 9, &missing);
                assert!(
                    app.window().len() <= 9,
                    "{n} entries, missing {missing:?}: window too tall"
                );
            }
        }
    }

    #[test]
    fn short_menu_is_not_padded_with_blanks() {
        // Only 4 rows exist, so the window is 4 rows: no phantom blank rows
        // below a menu that simply has fewer entries than the budget.
        let app = app(4, 9, &[]);
        assert_eq!(window_labels(&app).len(), 4);
    }

    /// The window must always be exactly the matched slice
    /// `[offset, offset + display)`, with nothing blank in it. This is the
    /// invariant the whole feature rests on, so it is checked after every kind
    /// of state change rather than per scenario.
    fn assert_window_is_the_slice(app: &App) {
        let labels = window_labels(app);
        let expected: Vec<String> = app
            .matched()
            .iter()
            .skip(app.offset)
            .take(app.display)
            .map(|&i| app.entries[i].label.clone())
            .collect();
        assert_eq!(labels, expected, "window drifted from the matched slice");
        assert!(labels.len() <= app.display, "window exceeds the budget");
    }

    #[test]
    fn highlight_skips_missing_rows() {
        let mut app = app(11, 9, &[3, 5, 7]);
        // Starts on e01 (window row 0).
        assert_eq!(app.highlight(), Some(0));
        // e02 is installed, so one step lands on it.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e02");
        assert_eq!(app.highlight(), Some(1));
        // e03 is not installed: the highlight goes to e04, the very next row.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e04");
        assert_eq!(app.highlight(), Some(2));
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn scrolling_moves_the_window_over_a_gap() {
        let mut app = app(20, 9, &[11, 13]);
        for _ in 0..9 {
            app.next();
            assert_window_is_the_slice(&app);
        }
        // e10 is the last row of the scrolled window (e02..e10).
        assert_eq!(app.current_selection().unwrap().label, "e10");
        assert_eq!(app.highlight(), Some(8));
        // Stepping again skips the uninstalled e11 and e13 without leaving a
        // hole: e12 and e14 take the rows the missing ones would have had.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e12");
        assert_eq!(app.highlight(), Some(8));
        assert_window_is_the_slice(&app);
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e14");
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn navigation_wraps_and_skips_every_missing_row() {
        let mut app = app(11, 9, &[3, 5, 7]);
        let order: Vec<String> = (0..8)
            .map(|_| {
                app.next();
                app.current_selection().unwrap().label
            })
            .collect();
        assert_eq!(
            order,
            vec!["e02", "e04", "e06", "e08", "e09", "e10", "e11", "e01"]
        );
        // ... and back the other way, which also wraps.
        let back: Vec<String> = (0..3)
            .map(|_| {
                app.previous();
                app.current_selection().unwrap().label
            })
            .collect();
        assert_eq!(back, vec!["e11", "e10", "e09"]);
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn query_resets_the_window_without_leaving_holes() {
        let mut app = app(20, 9, &[2, 3]);
        for _ in 0..12 {
            app.next();
        }
        assert!(app.offset > 0, "expected the list to have scrolled");
        app.type_char('e');
        app.type_char('0');
        // "e0" matches e01..e09; e02 and e03 are uninstalled, so the window is
        // the seven entries that are really there, with no blank rows.
        assert_eq!(
            window_labels(&app),
            vec!["e01", "e04", "e05", "e06", "e07", "e08", "e09"]
        );
        assert_eq!(app.highlight(), Some(0));
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn window_stays_the_slice_across_every_navigation() {
        // Broad sweep: the invariant must hold at every reachable offset.
        for missing in [vec![], vec![3, 5, 7], vec![1, 2, 3, 4, 5, 6, 7, 8]] {
            for n in 1..=20usize {
                for display in 1..=9usize {
                    let mut app = app(n, display, &missing);
                    assert_window_is_the_slice(&app);
                    for _ in 0..(n * 2) {
                        app.next();
                        assert_window_is_the_slice(&app);
                    }
                    for _ in 0..(n * 2) {
                        app.previous();
                        assert_window_is_the_slice(&app);
                    }
                }
            }
        }
    }

    #[test]
    fn no_installed_rows_highlights_nothing() {
        let app = app(4, 9, &[1, 2, 3, 4]);
        assert_eq!(app.highlight(), None);
        assert_eq!(app.current_selection(), None);
        assert_eq!(app.window().len(), 0);
    }
}
