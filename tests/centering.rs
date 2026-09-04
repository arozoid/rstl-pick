use ratatui::layout::Rect;

fn panel_for(width: u16, height: u16) -> Rect {
    let panel_w = 46u16.min(width.saturating_sub(2));
    let panel_h = 15u16.min(height.saturating_sub(2));
    let x = (width - panel_w) / 2;
    let y = (height - panel_h) / 2;
    Rect::new(x, y, panel_w, panel_h)
}

#[test]
fn panel_is_centered_at_many_widths() {
    let cases: &[(u16, u16, u16, u16)] = &[
        (120, 30, 46, 15),
        (100, 24, 46, 15),
        (90, 24, 46, 15),
        (80, 22, 46, 15),
        (70, 22, 46, 15),
        (60, 20, 46, 15),
        (55, 20, 46, 15),
        (50, 18, 46, 15),
        (48, 18, 46, 15),
        (47, 18, 45, 15),
        (40, 16, 38, 14),
    ];
    for &(w, h, exp_w, exp_h) in cases {
        let p = panel_for(w, h);
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
