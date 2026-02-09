use crate::app::App;
use crate::models::Mode;
use crate::ui::utils::{format_timer, hex_to_rgb};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let status_text = match app.mode {
        Mode::Time(limit) => {
            let seconds = if let Some(start) = app.start_time {
                let elapsed = start.elapsed().as_secs();
                limit.saturating_sub(elapsed)
            } else {
                limit
            };
            format_timer(seconds)
        }
        Mode::Words(total) => {
            let visible_words = app.input.split_whitespace().count();
            let mut total_typed = app.scrolled_word_count + visible_words;
            let is_finished = app.input.len() >= app.word_stream_string.len();
            if !app.input.ends_with(' ') && !is_finished && visible_words > 0 {
                total_typed = total_typed.saturating_sub(1);
            }
            format!("{}/{}", total_typed, total)
        }
        Mode::Quote(_) => {
            let visible_words = app.input.split_whitespace().count();
            let mut typed_words = app.scrolled_word_count + visible_words;
            let is_finished =
                app.input.len() >= app.word_stream_string.len() && app.quote_pool.is_empty();
            if !app.input.ends_with(' ') && !is_finished && visible_words > 0 {
                typed_words = typed_words.saturating_sub(1);
            }
            format!("{}/{}", typed_words, app.total_quote_words)
        }
    };

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

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Fill(1),
        ])
        .split(f.area());

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(80),
            Constraint::Fill(1),
        ])
        .split(vertical_layout[1]);

    let active_area = horizontal_layout[1];
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(0),
            Constraint::Min(1),
        ])
        .split(active_area);

    f.render_widget(
        Paragraph::new(status_text)
            .alignment(Alignment::Left)
            .style(
                Style::default()
                    .fg(hex_to_rgb(&app.theme.main))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        inner_chunks[0],
    );

    let mut visible_lines: Vec<Line> = Vec::new();
    let lines_to_show = app.visual_lines.iter().take(3);

    let mut global_char_idx = 0;
    let input_chars: Vec<char> = app.input.chars().collect();
    let text_area = inner_chunks[2];

    let color_correct = hex_to_rgb(&app.theme.text);
    let color_incorrect = hex_to_rgb(&app.theme.error);
    let color_future = hex_to_rgb(&app.theme.sub);

    // caret block is 'caret', text inside is 'sub' (for contrast)
    let color_cursor_bg = hex_to_rgb(&app.theme.caret);
    let color_cursor_fg = hex_to_rgb(&app.theme.sub);

    for line_str in lines_to_show {
        let mut spans: Vec<Span> = Vec::new();
        for (char_idx, c) in line_str.chars().enumerate() {
            let current_idx = global_char_idx + char_idx;

            let is_extra_char = if current_idx < app.display_mask.len() {
                app.display_mask[current_idx]
            } else {
                false
            };

            if current_idx < input_chars.len() {
                if is_extra_char {
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default()
                            .fg(color_incorrect)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ));
                } else {
                    let input_char = input_chars[current_idx];

                    if input_char == '\0' {
                        // missed == future/sub color
                        spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
                    } else if input_char == c {
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default()
                                .fg(color_correct)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default()
                                .fg(color_incorrect)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    }
                }
            } else if current_idx == input_chars.len() {

                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().bg(color_cursor_bg).fg(color_cursor_fg),
                ));
            } else {
                spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
            }
        }
        let line_end_idx = global_char_idx + line_str.chars().count();
        if input_chars.len() == line_end_idx {
            spans.push(Span::styled(
                " ",
                Style::default().bg(color_cursor_bg),
            ));
        }


        global_char_idx += line_str.len() + 1;
        visible_lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(visible_lines).alignment(Alignment::Left),
        text_area,
    );

    if app.show_ui {
        let footer = Paragraph::new("tab: restart | esc: quit")
            .style(Style::default().fg(hex_to_rgb(&app.theme.sub)))
            .alignment(Alignment::Center);
        f.render_widget(footer, Rect::new(0, f.area().height - 1, f.area().width, 1));
    }
}
