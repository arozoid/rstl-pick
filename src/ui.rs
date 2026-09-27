use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};

use crate::app::App;

/// The forest-green accent used throughout the UI (#6aa84f).
pub const ACCENT: Color = Color::Rgb(106, 168, 79);
/// Quiet foreground for secondary text.
const DIM: Color = Color::Rgb(140, 148, 132);
/// Panel background, slightly lifted from the page background.
const PANEL_BG: Color = Color::Rgb(24, 26, 22);
/// Page background (near-black with a green tint).
const PAGE_BG: Color = Color::Rgb(16, 18, 15);

/// Handle terminal resizing and redraw a single frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Page background.
    frame.render_widget(Block::default().style(Style::new().bg(PAGE_BG)), area);

    // A compact, centered panel sized for the configured number of visible
    // rows (`display`; extra matches scroll inside the list). Centering uses
    // explicit rect math ((avail - panel)/2 each side), which is exact and
    // immune to the flex/constraint quirks that mis-center blocks on
    // different terminal sizes (notably narrow/sub-1000px windows).
    let panel_w = 46u16.min(area.width.saturating_sub(2));
    // search box (3) + notice (1) + margins (2) + block border (2) + block
    // padding (2); without the border rows the list area comes out two rows
    // short of `display` and the last entries are scrolled out of view.
    let panel_h = (app.display as u16 + 10).min(area.height.saturating_sub(2));
    let page_x = (area.width - panel_w) / 2;
    let page_y = (area.height - panel_h) / 2;
    let panel = Rect::new(page_x, page_y, panel_w, panel_h);
    if panel.width < 30 || panel.height < 9 {
        render_too_small(frame, area);
        return;
    }

    let panel = panel.inner(Margin::new(1, 1));

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::new().bg(PANEL_BG))
        .border_style(Style::new().fg(ACCENT))
        .title(" pick ")
        .title_alignment(Alignment::Center)
        .title_style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))
        .padding(Padding::uniform(1));
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search box
            Constraint::Length(1), // notice / spacer
            Constraint::Min(1),    // list
        ])
        .split(inner);

    render_search(frame, layout[0], &app.query);
    render_notice(frame, layout[1], app.notice.as_deref());
    render_list(frame, layout[2], app);

    render_footer(frame, area);
}

/// Transient status line (e.g. "icon copied"); a blank spacer when idle.
fn render_notice(frame: &mut Frame, area: Rect, notice: Option<&str>) {
    if let Some(msg) = notice {
        let text = Paragraph::new(Line::from(Span::styled(msg, Style::new().fg(ACCENT))))
            .alignment(Alignment::Center);
        frame.render_widget(text, area);
    }
}

/// The search box: a bordered input with the current query and the terminal's
/// bar cursor at the end of the text (only while there is something to edit).
fn render_search(frame: &mut Frame, area: Rect, query: &str) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::new().fg(ACCENT)),
        Span::raw(query),
    ]))
    .block(
        Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::new().fg(ACCENT))
            .title(" search ")
            .title_style(Style::new().fg(DIM)),
    );
    frame.render_widget(input, area);

    // Position the real cursor right after the typed text so it reads as the
    // caret. An empty query leaves the cursor hidden.
    if !query.is_empty() {
        let cursor_x =
            area.x + 1 /* border */ + 2 /* spacing */ + Line::from(Span::raw(query)).width() as u16;
        frame.set_cursor_position((cursor_x, area.y + 1));
    }
}

/// The entry list: one row per display slot, blank where the entry is not
/// installed (see [`App::window`]).
fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let window = app.window();
    // Nothing to launch in view: either the query matched no row, or every row
    // it matched belongs to a program that is not installed. Both are dead
    // ends, so say so rather than leaving a blank panel.
    if !window.iter().any(Option::is_some) {
        let empty = Paragraph::new(Line::from(Span::styled("no match", Style::new().fg(DIM))))
            .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = window
        .iter()
        .map(|slot| {
            let Some(entry) = slot else {
                // A missing program keeps its row empty: the space is not
                // backfilled with the next entry of the menu.
                return ListItem::new(Line::from("")).style(Style::new().bg(PANEL_BG));
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", entry.key), Style::new().fg(ACCENT)),
                Span::raw(" "),
                Span::styled(
                    entry.label.clone(),
                    Style::new().fg(Color::Rgb(150, 165, 138)),
                ),
                Span::styled(format!("  {}", entry.hint), Style::new().fg(DIM)),
            ]))
            .style(Style::new().bg(PANEL_BG))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected);

    // A local state (instead of one owned by App) keeps the highlight a pure
    // function of the current window: the list never scrolls itself, the app
    // scrolls the window and reports where the highlight sits inside it.
    let mut state = ListState::default().with_offset(0);
    state.select(app.highlight());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let msg = Paragraph::new("Terminal too small, please enlarge the window.")
        .style(Style::new().fg(ACCENT))
        .alignment(Alignment::Center);
    frame.render_widget(msg, area);
}

/// Bottom-anchored keybinding hint.
fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(
        Line::from(vec![
            Span::styled("keys", Style::new().fg(ACCENT)),
            Span::raw(" launch   "),
            Span::styled("↑↓", Style::new().fg(ACCENT)),
            Span::raw(" move   "),
            Span::styled("enter", Style::new().fg(ACCENT)),
            Span::raw(" run   "),
            Span::styled("q", Style::new().fg(ACCENT)),
            Span::raw(" quit"),
        ])
        .alignment(Alignment::Center),
    )
    .style(Style::new().fg(DIM));
    frame.render_widget(
        footer,
        Rect::new(0, area.height.saturating_sub(1), area.width, 1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::App, config::MenuEntry};
    use ratatui::{backend::TestBackend, Terminal};

    const MISSING: &str = "rstl-pick-test-missing";
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

    /// Render once and return the terminal contents as text, row by row.
    fn render(app: &mut App, w: u16, h: u16) -> Vec<String> {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// The entry labels drawn in the list panel, in order. Blank rows (no
    /// label) are skipped, so the length is the number of *visible* rows.
    fn drawn_labels(rows: &[String]) -> Vec<String> {
        rows.iter()
            .filter_map(|r| {
                r.split_whitespace()
                    .find(|w| {
                        w.len() == 3
                            && w.starts_with('e')
                            && w[1..].chars().all(|c| c.is_ascii_digit())
                    })
                    .map(|w| w.to_string())
            })
            .collect()
    }

    #[test]
    fn missing_programs_leave_blank_rows() {
        // 11 entries, display 9, e03/e05/e07 not installed. The drawn panel
        // must show e01 e02 _ e04 _ e06 _ e08 e09 -- and must NOT pull e10/e11
        // up into the freed rows.
        let entries = (1..=11)
            .map(|i| entry(&format!("e{i:02}"), !matches!(i, 3 | 5 | 7)))
            .collect();
        let mut app = App::new(entries, 9);
        let rows = render(&mut app, 60, 24);

        let labels = drawn_labels(&rows);
        assert_eq!(
            labels,
            vec!["e01", "e02", "e04", "e06", "e08", "e09"],
            "blank rows should not shift later entries up"
        );
        // The panel still reserves all nine rows: three of them are blank.
        let list_rows = rows
            .iter()
            .filter(|r| r.trim_start().starts_with('│') && r.contains('│'))
            .count();
        assert!(
            list_rows >= 9,
            "expected 9 reserved list rows, found {list_rows}"
        );
    }

    #[test]
    fn all_installed_fills_every_row() {
        let entries = (1..=9).map(|i| entry(&format!("e{i:02}"), true)).collect();
        let mut app = App::new(entries, 9);
        let rows = render(&mut app, 60, 24);
        let labels = drawn_labels(&rows);
        assert_eq!(labels.len(), 9, "expected 9 filled rows, got {labels:?}");
    }

    #[test]
    fn highlight_moves_past_blank_rows() {
        let entries = (1..=11)
            .map(|i| entry(&format!("e{i:02}"), !matches!(i, 3 | 5 | 7)))
            .collect();
        let mut app = App::new(entries, 9);
        let first = render(&mut app, 60, 24);
        assert!(
            first.iter().any(|r| r.contains('▶')),
            "the first row should be highlighted on start"
        );
        // Two steps: e01 -> e02 -> e04 (e03 is uninstalled).
        app.next();
        app.next();
        let after = render(&mut app, 60, 24);
        let highlighted = after
            .iter()
            .find(|r| r.contains('▶'))
            .expect("a row should be highlighted");
        assert!(
            highlighted.contains("e04"),
            "expected e04 highlighted, got {highlighted:?}"
        );
    }

    #[test]
    fn no_match_still_renders() {
        let entries = (1..=5).map(|i| entry(&format!("e{i:02}"), true)).collect();
        let mut app = App::new(entries, 9);
        app.type_char('z');
        let rows = render(&mut app, 60, 24);
        assert!(rows.iter().any(|r| r.contains("no match")));
    }

    #[test]
    fn all_entries_missing_says_no_match() {
        // Nothing is installed: there is no row to show, so the panel reports
        // it instead of looking frozen.
        let entries = (1..=4).map(|i| entry(&format!("e{i:02}"), false)).collect();
        let mut app = App::new(entries, 9);
        let rows = render(&mut app, 60, 24);
        assert!(rows.iter().any(|r| r.contains("no match")), "{rows:?}");
        assert!(!rows.iter().any(|r| r.contains("e0")));
    }
}
