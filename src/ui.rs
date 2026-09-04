use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, Padding, Paragraph},
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
/// The search-box caret colour.
const CARET: Color = Color::Rgb(206, 216, 194);

/// Handle terminal resizing and redraw a single frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Page background.
    frame.render_widget(Block::default().style(Style::new().bg(PAGE_BG)), area);

    // A deliberately compact, centered panel: half-page vertical inset, with a
    // minimum usable width so the search box and one-line entries fit.
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(12),
            Constraint::Percentage(50),
        ])
        .split(area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(40),
            Constraint::Percentage(30),
        ])
        .split(page[1]);

    let panel = center[1];
    if panel.width < 40 || panel.height < 9 {
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
            Constraint::Length(3),  // search box
            Constraint::Length(1),  // spacer
            Constraint::Min(1),     // list
        ])
        .split(inner);

    render_search(frame, layout[0], &app.query);
    render_list(frame, layout[2], app);

    render_footer(frame, area);
}

/// The search box: a bordered input with the current query and a caret.
fn render_search(frame: &mut Frame, area: Rect, query: &str) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::new().fg(ACCENT)),
        Span::raw(query),
        Span::styled("▌", Style::new().fg(CARET)),
    ]))
    .block(
        Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::new().fg(ACCENT))
            .title(" search ")
            .title_style(Style::new().fg(DIM)),
    );
    frame.render_widget(input, area);
}

/// The filtered entry list.
fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible();
    if visible.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "no match",
            Style::new().fg(DIM),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let items = visible.iter().map(|it| {
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", it.key()), Style::new().fg(ACCENT)),
            Span::raw(" "),
            Span::styled(it.label(), Style::new().fg(Color::Rgb(150, 165, 138))),
            Span::styled(format!("  {}", it.run()), Style::new().fg(DIM)),
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
    let msg = Paragraph::new("Terminal too small — please enlarge the window.")
        .style(Style::new().fg(ACCENT))
        .alignment(Alignment::Center);
    frame.render_widget(msg, area);
}

/// Bottom-anchored keybinding hint.
fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(
        Line::from(vec![
            Span::styled("type", Style::new().fg(ACCENT)),
            Span::raw(" filter   "),
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