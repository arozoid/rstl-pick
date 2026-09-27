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

    /// Indices into `entries` of the rows matching the current search query,
    /// in config order.
    ///
    /// Availability is deliberately NOT applied here: the `display` budget is
    /// spent on these rows, so a row whose tools are not installed still
    /// consumes a display row (it is drawn blank) instead of the row being
    /// backfilled from further down the menu.
    fn matched(&self) -> Vec<usize> {
        let q = self.query.trim().to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| q.is_empty() || entry.label.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    /// The rows currently on screen: up to `display` matched rows starting at
    /// the scroll offset. `None` marks a row whose entry is not installed --
    /// the slot stays blank rather than being filled with the next entry.
    pub fn window(&self) -> Vec<Option<MenuEntry>> {
        let matched = self.matched();
        let start = self.offset.min(matched.len());
        matched[start..]
            .iter()
            .take(self.display)
            .map(|&i| {
                if self.available[i] {
                    Some(self.entries[i].clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Position of the highlighted row within [`App::window`], if the
    /// highlighted row is on screen.
    pub fn highlight(&self) -> Option<usize> {
        let sel = self.selected?;
        (self.offset <= sel && sel < self.offset + self.display).then_some(sel - self.offset)
    }

    /// The currently highlighted entry (if the filtered list is non-empty).
    pub fn current_selection(&self) -> Option<MenuEntry> {
        let i = *self.matched().get(self.selected?)?;
        self.available[i].then(|| self.entries[i].clone())
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

    /// Reset the selection to the first installed matching row, scrolled top.
    fn jump_top(&mut self) {
        self.offset = 0;
        self.selected = self.first_installed();
    }

    /// Index of the first matched row whose entry is installed.
    fn first_installed(&self) -> Option<usize> {
        self.matched().iter().position(|&i| self.available[i])
    }

    /// Move the selection by `dir` matched rows, skipping rows whose entry is
    /// not installed, and scroll the window to keep it on screen.
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
        // Walk at most n rows: the first installed row found is the target
        // (wrapping), or nothing is installed anywhere in the menu.
        for step in 1..=n {
            let i = (from + dir * step).rem_euclid(n) as usize;
            if self.available[matched[i]] {
                self.selected = Some(i);
                self.scroll_into_view(i);
                return;
            }
        }
        self.selected = None;
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

    /// The labels currently in the window, with `None` for a blank row.
    fn window_labels(app: &App) -> Vec<Option<String>> {
        app.window()
            .iter()
            .map(|slot| slot.as_ref().map(|e| e.label.clone()))
            .collect()
    }

    #[test]
    fn missing_row_is_blank_and_not_backfilled() {
        // display 9 of 11, three not installed: their rows stay empty and the
        // two rows below the budget (e10, e11) are NOT pulled up to fill them.
        let app = app(11, 9, &[3, 5, 7]);
        assert_eq!(
            window_labels(&app),
            vec![
                Some("e01".into()),
                Some("e02".into()),
                None,
                Some("e04".into()),
                None,
                Some("e06".into()),
                None,
                Some("e08".into()),
                Some("e09".into()),
            ]
        );
    }

    #[test]
    fn all_installed_shows_a_full_window() {
        let app = app(20, 9, &[]);
        assert_eq!(
            window_labels(&app),
            (1..=9)
                .map(|i| Some(format!("e{i:02}")))
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

    /// The window must always be exactly the matched rows in
    /// `[offset, offset + display)`, with a blank wherever the entry is not
    /// installed. This is the invariant the whole feature rests on, so it is
    /// checked after every kind of state change rather than per scenario.
    fn assert_window_is_the_slice(app: &App) {
        let labels = window_labels(app);
        let matched: Vec<Option<String>> = app
            .matched()
            .iter()
            .skip(app.offset)
            .take(app.display)
            .map(|&i| app.available[i].then(|| app.entries[i].label.clone()))
            .collect();
        assert_eq!(labels, matched, "window drifted from the matched slice");
        assert!(labels.len() <= app.display, "window exceeds the budget");
    }

    #[test]
    fn highlight_skips_blank_rows() {
        let mut app = app(11, 9, &[3, 5, 7]);
        // Starts on e01 (window row 0).
        assert_eq!(app.highlight(), Some(0));
        // e02 is installed, so one step lands on it.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e02");
        assert_eq!(app.highlight(), Some(1));
        // e03 is not installed: the highlight jumps its blank row to e04.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e04");
        assert_eq!(app.highlight(), Some(3));
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn scrolling_moves_the_window_and_keeps_gaps() {
        let mut app = app(20, 9, &[11, 13]);
        for _ in 0..9 {
            app.next();
            assert_window_is_the_slice(&app);
        }
        // e10 is the last row of the scrolled window (e02..e10).
        assert_eq!(app.current_selection().unwrap().label, "e10");
        assert_eq!(app.highlight(), Some(8));
        // Stepping again skips the uninstalled e11 and e13 without ever
        // backfilling their slots from further down the menu.
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e12");
        assert_window_is_the_slice(&app);
        app.next();
        assert_eq!(app.current_selection().unwrap().label, "e14");
        let labels = window_labels(&app);
        assert!(
            labels.contains(&None),
            "e13's row should still be blank somewhere in the window: {labels:?}"
        );
        assert_window_is_the_slice(&app);
    }

    #[test]
    fn navigation_wraps_and_skips_every_blank() {
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
    fn query_resets_the_window_and_keeps_blank_slots() {
        let mut app = app(20, 9, &[2, 3]);
        for _ in 0..12 {
            app.next();
        }
        assert!(app.offset > 0, "expected the list to have scrolled");
        app.type_char('e');
        app.type_char('0');
        // "e0" matches e01..e09; e02 and e03 are uninstalled, so their rows
        // are blank instead of pulling e04..e09 up by two.
        assert_eq!(
            window_labels(&app),
            vec![
                Some("e01".into()),
                None,
                None,
                Some("e04".into()),
                Some("e05".into()),
                Some("e06".into()),
                Some("e07".into()),
                Some("e08".into()),
                Some("e09".into()),
            ]
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
        assert_eq!(app.window().len(), 4);
    }
}
