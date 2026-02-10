use ratatui::style::Color;
use ratatui::{
    layout::{Alignment, Rect, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::app::App;

pub fn hex_to_rgb(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}

pub fn format_timer(seconds: u64) -> String {
    if seconds >= 60 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{}", seconds)
    }
}

pub fn get_quote_length_category(char_count: usize) -> &'static str {
    if char_count <= 100 {
        "short"
    } else if char_count <= 300 {
        "medium"
    } else if char_count <= 600 {
        "long"
    } else {
        "very long"
    }
}

pub fn render_header(f: &mut Frame, app: &App) {
    let mut header_spans = Vec::new();
    // use 'main' for active brand, 'sub' for inactive
    let brand_color = if app.show_ui {
        hex_to_rgb(&app.theme.main)
    } else {
        hex_to_rgb(&app.theme.sub)
    };

    header_spans.push(Span::styled(
        "typa",
        Style::default()
            .fg(brand_color)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ));

    if app.show_ui {
        header_spans.push(Span::styled(
            format!(" | mode: {:?}", app.mode),
            Style::default().fg(hex_to_rgb(&app.theme.sub)),
        ));
    }

    let header_row_area = Rect::new(0, 1, f.area().width, 1);

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(82),
            Constraint::Fill(1),
        ])
        .split(header_row_area);

    f.render_widget(Paragraph::new(Line::from(header_spans)), header_layout[1]);
}

pub fn render_footer(f: &mut Frame, app: &App) {
    if app.show_ui {
        let footer = Paragraph::new("tab: restart | esc: quit")
            .style(Style::default().fg(hex_to_rgb(&app.theme.sub)))
            .alignment(Alignment::Center);
        f.render_widget(footer, Rect::new(0, f.area().height - 1, f.area().width, 1));
    }
}
