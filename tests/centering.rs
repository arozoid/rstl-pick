use ratatui::layout::Rect;

fn panel_for(width: u16, height: u16, display: usize) -> Rect {
    let panel_w = 46u16.min(width.saturating_sub(2));
    // search box (3) + notice (1) + margins (2) + block padding (2)
    let panel_h = (display as u16 + 8).min(height.saturating_sub(2));
    let x = (width - panel_w) / 2;
    let y = (height - panel_h) / 2;
    Rect::new(x, y, panel_w, panel_h)
}

#[test]
fn panel_is_centered_at_many_widths() {
    let cases: &[(u16, u16, usize, u16, u16)] = &[
        (120, 30, 9, 46, 17),
        (100, 24, 9, 46, 17),
        (90, 24, 9, 46, 17),
        (80, 22, 9, 46, 17),
        (70, 22, 9, 46, 17),
        (60, 20, 9, 46, 17),
        (55, 20, 9, 46, 17),
        (50, 18, 9, 46, 16),
        (48, 18, 9, 46, 16),
        (47, 18, 9, 45, 16),
        (40, 16, 9, 38, 14),
        // a smaller display shrinks the panel height
        (100, 24, 4, 46, 12),
        (70, 22, 2, 46, 10),
        // a big display is clamped by the terminal height
        (100, 24, 20, 46, 22),
    ];
    for &(w, h, display, exp_w, exp_h) in cases {
        let p = panel_for(w, h, display);
        let left = p.x;
        let right = w - (p.x + p.width);
        let top = p.y;
        let bottom = h - (p.y + p.height);
        assert!(
            (left as i32 - right as i32).abs() <= 1,
            "width {}: not centered horizontally (left={} right={}) panel_w={}",
            w,
            left,
            right,
            p.width
        );
        assert!(
            (top as i32 - bottom as i32).abs() <= 1,
            "height {}: not centered vertically (top={} bottom={})",
            h,
            top,
            bottom
        );
        assert_eq!(p.width, exp_w, "width {}: unexpected panel width", w);
        assert_eq!(p.height, exp_h, "height {}: unexpected panel height", h);
        assert!(p.x + p.width <= w, "width {}: panel overflows", w);
        assert!(p.y + p.height <= h, "height {}: panel overflows", h);
        println!(
            "{}x{}: panel x={} y={} w={} h={} | ctr L/R={}/{} T/B={}/{}",
            w, h, p.x, p.y, p.width, p.height, left, right, top, bottom
        );
    }
}