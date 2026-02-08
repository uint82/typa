use crate::app::{App, AppState, Mode, QuoteSelector};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

fn hex_to_rgb(hex: &str) -> Color {
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

fn format_timer(seconds: u64) -> String {
    if seconds >= 60 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{}", seconds)
    }
}

pub fn render(f: &mut Frame, app: &App) {
    let bg_color = hex_to_rgb(&app.theme.bg);
    f.render_widget(
        Block::default().style(Style::default().bg(bg_color)),
        f.area(),
    );

    if app.state == AppState::Finished {
        draw_results(f, app);
    } else {
        draw_test(f, app);
    }
}

fn draw_results(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(14),
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
        .split(chunks[1]);

    let area = horizontal_layout[1];

    let block = Block::default()
        .title(" Result ")
        .borders(Borders::ALL)
        .style(Style::default().fg(hex_to_rgb(&app.theme.sub_alt)));

    f.render_widget(block, area);

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let sub_color = hex_to_rgb(&app.theme.sub);
    let main_color = hex_to_rgb(&app.theme.main);

    let wpm_line = Line::from(vec![
        Span::styled("wpm: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_wpm),
            Style::default().fg(main_color).add_modifier(ratatui::style::Modifier::BOLD)
        ),
    ]);

    f.render_widget(
        Paragraph::new(wpm_line).alignment(Alignment::Center),
        inner_layout[0]
    );

    let stats_line = Line::from(vec![
        Span::styled("acc: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}%", app.final_accuracy),
            Style::default().fg(main_color)
        ),
        Span::styled(" | raw: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_raw_wpm),
            Style::default().fg(main_color)
        ),
    ]);

    f.render_widget(
        Paragraph::new(stats_line).alignment(Alignment::Center),
        inner_layout[1],
    );

    let mut total_correct = app.st_correct;
    let mut total_incorrect = app.st_incorrect;
    let mut total_extra = app.st_extra;
    let mut total_missed = app.st_missed;

    for (i, c) in app.input.chars().enumerate() {
        if i < app.display_mask.len() {
            let is_extra = app.display_mask[i];
            if is_extra {
                total_extra += 1;
            } else {
                let target = app.display_string.chars().nth(i).unwrap_or(' ');
                if c == '\0' {
                    total_missed += 1;
                } else if c == target {
                    total_correct += 1;
                } else {
                    total_incorrect += 1;
                }
            }
        }
    }

    let chars_line = Line::from(vec![
        Span::styled("cor: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_correct), Style::default().fg(main_color)),
        Span::styled(" | inc: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_incorrect), Style::default().fg(main_color)),
        Span::styled(" | ext: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_extra), Style::default().fg(main_color)),
        Span::styled(" | mis: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_missed), Style::default().fg(main_color)),
        Span::styled(" | time: ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.1}s", app.final_time), Style::default().fg(main_color)),
    ]);

    f.render_widget(
        Paragraph::new(chars_line).alignment(Alignment::Center),
        inner_layout[2],
    );

    let mode_str = match &app.mode {
        Mode::Time(t) => format!("time {}", t),
        Mode::Words(w) => format!("word {}", w),
        Mode::Quote(q) => match q {
            QuoteSelector::Id(_) => "quote".to_string(),
            QuoteSelector::Category(len) => format!("quote {:?}", len).to_lowercase(),
        },
    };
    let mut type_parts = vec![mode_str, app.word_data.name.clone()];
    if app.use_punctuation {
        type_parts.push("punctuation".to_string());
    }
    if app.use_numbers {
        type_parts.push("number".to_string());
    }

    let type_value = type_parts.join(" ");

    let type_line = Line::from(vec![
        Span::styled("test type: ", Style::default().fg(sub_color)),
        Span::styled(type_value, Style::default().fg(main_color)),
    ]);

    f.render_widget(
        Paragraph::new(type_line).alignment(Alignment::Center),
        inner_layout[3],
    );

    if !app.current_quote_source.is_empty() {
        let source_line = Line::from(vec![
            Span::styled("source: ", Style::default().fg(sub_color)),
            Span::styled(&app.current_quote_source, Style::default().fg(main_color)),
        ]);

        f.render_widget(
            Paragraph::new(source_line).alignment(Alignment::Center),
            inner_layout[5],
        );
    }

    f.render_widget(
        Paragraph::new("Press TAB to Restart")
            .alignment(Alignment::Center)
            .style(Style::default().fg(sub_color)),
        inner_layout[6],
    );
}

fn draw_test(f: &mut Frame, app: &App) {
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
            let is_finished = app.input.len() >= app.word_stream_string.len() && app.quote_pool.is_empty();
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
        " crabtype",
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
    let mut cursor_screen_pos: Option<(u16, u16)> = None;
    let text_area = inner_chunks[2];

    let color_correct = hex_to_rgb(&app.theme.text);
    let color_incorrect = hex_to_rgb(&app.theme.error);
    let color_future = hex_to_rgb(&app.theme.sub);

    // caret block is 'caret', text inside is 'bg' (for contrast)
    let color_cursor_bg = hex_to_rgb(&app.theme.caret);
    let color_cursor_fg = hex_to_rgb(&app.theme.bg);

    for (line_idx, line_str) in lines_to_show.enumerate() {
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
                        spans.push(Span::styled(
                            c.to_string(),
                            Style::default().fg(color_future),
                        ));
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
                let screen_x = text_area.x + char_idx as u16;
                let screen_y = text_area.y + line_idx as u16;
                cursor_screen_pos = Some((screen_x, screen_y));

                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().bg(color_cursor_bg).fg(color_cursor_fg),
                ));
            } else {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(color_future),
                ));
            }
        }

        // handle cursor at end of line
        let line_end_idx = global_char_idx + line_str.len();
        if input_chars.len() == line_end_idx {
            let screen_x = text_area.x + line_str.chars().count() as u16;
            let screen_y = text_area.y + line_idx as u16;
            cursor_screen_pos = Some((screen_x, screen_y));
        }

        global_char_idx += line_str.len() + 1;
        visible_lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(visible_lines).alignment(Alignment::Left),
        text_area,
    );

    if let Some((x, y)) = cursor_screen_pos {
        f.set_cursor_position((x, y));
    }

    if app.show_ui {
        let footer = Paragraph::new("tab: restart | esc: quit")
            .style(Style::default().fg(hex_to_rgb(&app.theme.sub)))
            .alignment(Alignment::Center);
        f.render_widget(footer, Rect::new(0, f.area().height - 1, f.area().width, 1));
    }
}
