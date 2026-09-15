use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{App, PickerItem};

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

    // A compact, centered panel sized to fit every entry at once (no
    // scrolling): search box + all seven one-line items are always visible.
    // Centering uses explicit rect math ((avail - panel)/2 each side), which
    // is exact and immune to the flex/constraint quirks that mis-center
    // blocks on different terminal sizes (notably narrow/sub-1000px windows).
    let panel_w = 46u16.min(area.width.saturating_sub(2));
    let panel_h = 17u16.min(area.height.saturating_sub(2));
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

/// The filtered entry list.
fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible();
    if visible.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("no match", Style::new().fg(DIM))))
            .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let items = visible.iter().map(|it| {
        let hint = if *it == PickerItem::Files {
            app.file_manager.unwrap_or("spf / rovr / lf").to_string()
        } else {
            it.hint().to_string()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", it.key()), Style::new().fg(ACCENT)),
            Span::raw(" "),
            Span::styled(it.label(), Style::new().fg(Color::Rgb(150, 165, 138))),
            Span::styled(format!("  {}", hint), Style::new().fg(DIM)),
        ]))
        .style(Style::new().bg(PANEL_BG))
    });

    let list = List::new(items)
        .highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected);

    frame.render_stateful_widget(list, area, &mut app.list);
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
