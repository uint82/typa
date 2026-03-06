use crate::app::App;
use crate::models::Mode;
use crate::models::AppState;
use crate::ui::utils::{format_timer, hex_to_rgb, render_header, render_footer};
use crate::utils::strings;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let status_text = match app.config.mode {
        Mode::Time(limit) => {
            let seconds = if let Some(start) = app.test.start_time {
                let elapsed = start.elapsed().as_secs();
                limit.saturating_sub(elapsed)
            } else {
                limit
            };
            format_timer(seconds)
        }
        Mode::Words(total) => {
            let visible_words = app.test.input.split_whitespace().count();
            let mut total_typed = app.test.scrolled_word_count + visible_words;
            let is_finished = app.test.aligned_input.len() >= app.test.word_stream_string.chars().count();
            if !app.test.input.ends_with(' ') && !is_finished && visible_words > 0 {
                total_typed = total_typed.saturating_sub(1);
            }
            format!("{}/{}", total_typed, total)
        }
        Mode::Quote(_) => {
            let visible_words = app.test.input.split_whitespace().count();
            let mut typed_words = app.test.scrolled_word_count + visible_words;
            let is_finished =
                app.test.aligned_input.len() >= app.test.word_stream_string.chars().count() && app.test.quote_pool.is_empty();
            if !app.test.input.ends_with(' ') && !is_finished && visible_words > 0 {
                typed_words = typed_words.saturating_sub(1);
            }
            format!("{}/{}", typed_words, app.test.total_quote_words)
        }
    };

    render_header(f, app);

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
                    .fg(hex_to_rgb(&app.config.theme.main))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        inner_chunks[0],
    );

    let elapsed_ms = app.test.caret_epoch.elapsed().as_millis();
    const BLINK_PERIOD_MS: u128 = 530;

    let caret_visible = app.test.state == AppState::Running
        || (elapsed_ms / BLINK_PERIOD_MS) % 2 == 0;

    let mut visible_lines: Vec<Line> = Vec::new();
    let lines_to_show = app.test.visual_lines.iter().take(3);

    let mut global_char_idx = 0;
    let input_chars = &app.test.aligned_input;
    let text_area = inner_chunks[2];

    let color_correct = hex_to_rgb(&app.config.theme.text);
    let color_incorrect = hex_to_rgb(&app.config.theme.error);
    let color_future = hex_to_rgb(&app.config.theme.sub);

    // caret block is 'caret', text inside is 'sub' (for contrast)
    let color_cursor_bg = hex_to_rgb(&app.config.theme.caret);
    let color_cursor_fg = hex_to_rgb(&app.config.theme.sub);

    for line_str in lines_to_show {
        let mut spans: Vec<Span> = Vec::new();
        for (char_idx, c) in line_str.chars().enumerate() {
            let current_idx = global_char_idx + char_idx;

            let is_extra_char = if current_idx < app.test.display_mask.len() {
                app.test.display_mask[current_idx]
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
                    } else if strings::are_characters_visually_equal(input_char, c) {
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
                    if caret_visible {
                        Style::default().bg(color_cursor_bg).fg(color_cursor_fg)
                    } else {
                        Style::default().fg(color_future)
                    },
                ));
            } else {
                spans.push(Span::styled(c.to_string(), Style::default().fg(color_future)));
            }
        }
        let line_end_idx = global_char_idx + line_str.chars().count();
        if input_chars.len() == line_end_idx && caret_visible {
            spans.push(Span::styled(
                " ",
                Style::default().bg(color_cursor_bg),
            ));
        }


        global_char_idx += line_str.chars().count() + 1;
        visible_lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(visible_lines).alignment(Alignment::Left),
        text_area,
    );

    render_footer(f, app);
}
