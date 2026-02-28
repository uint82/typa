use crate::app::App;
use crate::models::{Mode, QuoteSelector};
use crate::ui::utils::{hex_to_rgb, get_quote_length_category, render_header, render_footer};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    render_header(f, app);

    let main_area = Rect::new(
        0,
        2,
        f.area().width,
        f.area().height.saturating_sub(3),
    );

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(get_content_height(f.area().height)),
            Constraint::Fill(1),
        ])
        .split(main_area);

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(82),
            Constraint::Fill(1),
        ])
        .split(vertical_chunks[1]);

    let area = horizontal_layout[1];

    let available_height = area.height;

    let layout_mode = if available_height >= 25 {
        LayoutMode::Full
    } else if available_height >= 18 {
        LayoutMode::Compact
    } else {
        LayoutMode::UltraCompact
    };

    let content_layout = match layout_mode {
        LayoutMode::Full => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(7),
                    Constraint::Length(1),
                    Constraint::Min(12),
                    Constraint::Length(1),
                    Constraint::Length(3),
                ])
                .split(area)
        },
        LayoutMode::Compact => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(5),
                    Constraint::Min(8),
                    Constraint::Length(3),
                ])
                .split(area)
        },
        LayoutMode::UltraCompact => {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(area)
        }
    };

    let bg_color    = hex_to_rgb(&app.theme.bg);
    let sub_color   = hex_to_rgb(&app.theme.sub);
    let main_color  = hex_to_rgb(&app.theme.main);
    let error_color = hex_to_rgb(&app.theme.error);

    match layout_mode {
        LayoutMode::Full => {
            draw_test_type_header(f, app, content_layout[0], sub_color, main_color);
            draw_full_stats_card(f, app, content_layout[2], sub_color, main_color);
            draw_chart(f, app, content_layout[4], bg_color, sub_color, main_color, error_color, true);
            draw_full_footer(f, app, content_layout[6], sub_color, main_color);
        },
        LayoutMode::Compact => {
            draw_test_type_header(f, app, content_layout[0], sub_color, main_color);
            draw_compact_stats_card(f, app, content_layout[1], sub_color, main_color);
            draw_chart(f, app, content_layout[2], bg_color, sub_color, main_color, error_color, true);
            draw_compact_footer(f, app, content_layout[3], sub_color, main_color);
        },
        LayoutMode::UltraCompact => {
            draw_test_type_header(f, app, content_layout[0], sub_color, main_color);
            draw_ultra_compact_stats(f, app, content_layout[1], sub_color, main_color);
            draw_chart(f, app, content_layout[2], bg_color, sub_color, main_color, error_color, false);
            draw_ultra_compact_footer(f, app, content_layout[3], sub_color, main_color);
        }
    }

    render_footer(f, app);
}

#[derive(Debug, Clone, Copy)]
enum LayoutMode {
    Full,
    Compact,
    UltraCompact,
}

fn get_content_height(terminal_height: u16) -> u16 {
    let available = terminal_height.saturating_sub(5);
    available.max(12).min(50)
}

fn draw_test_type_header(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    _main_color: ratatui::style::Color,
) {
    let mode_str = match &app.mode {
        Mode::Time(t) => format!("time {}", t),
        Mode::Words(w) => format!("word {}", w),
        Mode::Quote(q) => match q {
            QuoteSelector::Id(_) => format!("quote {}", get_quote_length_category(app.original_quote_length)),
            QuoteSelector::Category(len) => {
                let s = format!("{:?}", len).to_lowercase();
                format!("quote {}", if s == "all" { get_quote_length_category(app.original_quote_length) } else { &s })
            }
        },
    };

    let mut type_parts = vec![mode_str, app.word_data.name.clone()];
    if app.use_punctuation { type_parts.push("punctuation".to_string()); }
    if app.use_numbers     { type_parts.push("number".to_string()); }

    let header = Line::from(vec![
        Span::styled(type_parts.join(" "), Style::default().fg(sub_color)),
    ]);
    f.render_widget(Paragraph::new(header).alignment(Alignment::Center), area);
}

fn draw_full_stats_card(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let border_top = "─".repeat(area.width as usize);
    f.render_widget(
        Paragraph::new(border_top).style(Style::default().fg(sub_color)),
        rows[0]
    );

    let wpm_line = Line::from(vec![
        Span::styled("  WPM: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_wpm),
            Style::default()
                .fg(main_color)
                .add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::UNDERLINED),
        ),
    ]);
    f.render_widget(Paragraph::new(wpm_line).alignment(Alignment::Center), rows[1]);

    let acc_line = Line::from(vec![
        Span::styled("  Accuracy: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.2}%", app.final_accuracy),
            Style::default()
                .fg(main_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(acc_line).alignment(Alignment::Center), rows[2]);

    let secondary = Line::from(vec![
        Span::styled("raw ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}", app.final_raw_wpm), Style::default().fg(main_color)),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("time ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.1}s", app.final_time), Style::default().fg(main_color)),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("consistency ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}%", app.final_consistency), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(secondary).alignment(Alignment::Center), rows[4]);

    let (_, _, vis_raw_cor, vis_raw_inc, vis_raw_ext, vis_raw_mis) =
        app.calculate_custom_stats_for_slice(&app.aligned_input, &app.display_string, &app.display_mask);

    let total_chars = app.st_correct + vis_raw_cor + app.st_incorrect + vis_raw_inc +
                      app.st_extra + vis_raw_ext + app.st_missed + vis_raw_mis;

    let acc_breakdown = Line::from(vec![
        Span::styled("correct ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_correct + vis_raw_cor), Style::default().fg(main_color)),
        Span::styled(" / ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_chars), Style::default().fg(main_color)),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("errors ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{}", app.st_incorrect + vis_raw_inc + app.st_extra + vis_raw_ext + app.st_missed + vis_raw_mis),
            Style::default().fg(main_color)
        ),
    ]);
    f.render_widget(Paragraph::new(acc_breakdown).alignment(Alignment::Center), rows[5]);

    let border_bottom = "─".repeat(area.width as usize);
    f.render_widget(
        Paragraph::new(border_bottom).style(Style::default().fg(sub_color)),
        rows[6]
    );
}

fn draw_compact_stats_card(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let primary = Line::from(vec![
        Span::styled("WPM ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_wpm),
            Style::default().fg(main_color).add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("Acc ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.2}%", app.final_accuracy), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(primary).alignment(Alignment::Center), rows[0]);

    let secondary = Line::from(vec![
        Span::styled("raw ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}", app.final_raw_wpm), Style::default().fg(main_color)),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("time ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.1}s", app.final_time), Style::default().fg(main_color)),
        Span::styled("  │  ", Style::default().fg(sub_color)),
        Span::styled("con ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}%", app.final_consistency), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(secondary).alignment(Alignment::Center), rows[1]);

    let (_, _, vis_raw_cor, vis_raw_inc, vis_raw_ext, vis_raw_mis) =
        app.calculate_custom_stats_for_slice(&app.aligned_input, &app.display_string, &app.display_mask);

    let total_chars = app.st_correct + vis_raw_cor + app.st_incorrect + vis_raw_inc +
                      app.st_extra + vis_raw_ext + app.st_missed + vis_raw_mis;
    let errors = app.st_incorrect + vis_raw_inc + app.st_extra + vis_raw_ext + app.st_missed + vis_raw_mis;

    let breakdown = Line::from(vec![
        Span::styled(format!("{}/{}", app.st_correct + vis_raw_cor, total_chars), Style::default().fg(main_color)),
        Span::styled(" correct  │  ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", errors), Style::default().fg(main_color)),
        Span::styled(" errors", Style::default().fg(sub_color)),
    ]);
    f.render_widget(Paragraph::new(breakdown).alignment(Alignment::Center), rows[2]);

    let char_detail = Line::from(vec![
        Span::styled("cor ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_correct + vis_raw_cor), Style::default().fg(main_color)),
        Span::styled(" │ inc ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_incorrect + vis_raw_inc), Style::default().fg(main_color)),
        Span::styled(" │ ext ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_extra + vis_raw_ext), Style::default().fg(main_color)),
        Span::styled(" │ mis ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_missed + vis_raw_mis), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(char_detail).alignment(Alignment::Center), rows[3]);

    let total_ks = app.live_correct_keystrokes + app.live_incorrect_keystrokes;
    let ks_line = Line::from(vec![
        Span::styled("keystrokes ", Style::default().fg(sub_color)),
        Span::styled(format!("{}/{}", app.live_correct_keystrokes, total_ks), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(ks_line).alignment(Alignment::Center), rows[4]);
}

fn draw_ultra_compact_stats(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let (_, _, vis_raw_cor, vis_raw_inc, vis_raw_ext, vis_raw_mis) =
        app.calculate_custom_stats_for_slice(&app.aligned_input, &app.display_string, &app.display_mask);

    let total_chars = app.st_correct + vis_raw_cor + app.st_incorrect + vis_raw_inc +
                      app.st_extra + vis_raw_ext + app.st_missed + vis_raw_mis;

    let primary = Line::from(vec![
        Span::styled("wpm ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}", app.final_wpm), Style::default().fg(main_color).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(sub_color)),
        Span::styled("acc ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.1}%", app.final_accuracy), Style::default().fg(main_color)),
        Span::styled(" │ ", Style::default().fg(sub_color)),
        Span::styled("raw ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}", app.final_raw_wpm), Style::default().fg(main_color)),
        Span::styled(" │ ", Style::default().fg(sub_color)),
        Span::styled("con ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.0}%", app.final_consistency), Style::default().fg(main_color)),
        Span::styled(" │ ", Style::default().fg(sub_color)),
        Span::styled(format!("{:.1}s", app.final_time), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(primary).alignment(Alignment::Center), rows[0]);

    let char_line = Line::from(vec![
        Span::styled(format!("{}/{}", app.st_correct + vis_raw_cor, total_chars), Style::default().fg(main_color)),
        Span::styled(" cor │ ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_incorrect + vis_raw_inc), Style::default().fg(main_color)),
        Span::styled(" inc │ ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_extra + vis_raw_ext), Style::default().fg(main_color)),
        Span::styled(" ext │ ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_missed + vis_raw_mis), Style::default().fg(main_color)),
        Span::styled(" mis", Style::default().fg(sub_color)),
    ]);
    f.render_widget(Paragraph::new(char_line).alignment(Alignment::Center), rows[1]);

    let total_ks = app.live_correct_keystrokes + app.live_incorrect_keystrokes;
    let ks_line = Line::from(vec![
        Span::styled("keystroke ", Style::default().fg(sub_color)),
        Span::styled(format!("{}/{}", app.live_correct_keystrokes, total_ks), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(ks_line).alignment(Alignment::Center), rows[2]);
}

fn draw_full_footer(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let (_, _, vis_raw_cor, vis_raw_inc, vis_raw_ext, vis_raw_mis) =
        app.calculate_custom_stats_for_slice(&app.aligned_input, &app.display_string, &app.display_mask);

    let char_detail = Line::from(vec![
        Span::styled("chars: ", Style::default().fg(sub_color)),
        Span::styled("cor ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_correct + vis_raw_cor), Style::default().fg(main_color)),
        Span::styled(" │ inc ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_incorrect + vis_raw_inc), Style::default().fg(main_color)),
        Span::styled(" │ ext ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_extra + vis_raw_ext), Style::default().fg(main_color)),
        Span::styled(" │ mis ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.st_missed + vis_raw_mis), Style::default().fg(main_color)),
    ]);
    f.render_widget(Paragraph::new(char_detail).alignment(Alignment::Center), rows[0]);

    let total_ks = app.live_correct_keystrokes + app.live_incorrect_keystrokes;
    let ks_acc = if total_ks > 0 {
        (app.live_correct_keystrokes as f64 / total_ks as f64) * 100.0
    } else {
        100.0
    };

    let keystroke_detail = Line::from(vec![
        Span::styled("keystrokes: ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", app.live_correct_keystrokes), Style::default().fg(main_color)),
        Span::styled(" / ", Style::default().fg(sub_color)),
        Span::styled(format!("{}", total_ks), Style::default().fg(main_color)),
        Span::styled(format!(" ({:.1}%)", ks_acc), Style::default().fg(sub_color)),
    ]);
    f.render_widget(Paragraph::new(keystroke_detail).alignment(Alignment::Center), rows[1]);

    if !app.current_quote_source.is_empty() {
        let source = Line::from(vec![
            Span::styled("source: ", Style::default().fg(sub_color)),
            Span::styled(&app.current_quote_source, Style::default().fg(main_color)),
        ]);
        f.render_widget(Paragraph::new(source).alignment(Alignment::Center), rows[2]);
    }
}

fn draw_compact_footer(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let total_ks = app.live_correct_keystrokes + app.live_incorrect_keystrokes;
    let ks_acc = if total_ks > 0 {
        (app.live_correct_keystrokes as f64 / total_ks as f64) * 100.0
    } else {
        100.0
    };

    let ks_line = Line::from(vec![
        Span::styled("keystrokes ", Style::default().fg(sub_color)),
        Span::styled(format!("{}/{}", app.live_correct_keystrokes, total_ks), Style::default().fg(main_color)),
        Span::styled(format!(" ({:.1}%)", ks_acc), Style::default().fg(sub_color)),
    ]);
    f.render_widget(Paragraph::new(ks_line).alignment(Alignment::Center), rows[0]);

    if !app.current_quote_source.is_empty() {
        let source = Line::from(vec![
            Span::styled("― ", Style::default().fg(sub_color)),
            Span::styled(&app.current_quote_source, Style::default().fg(main_color)),
        ]);
        f.render_widget(Paragraph::new(source).alignment(Alignment::Center), rows[1]);
    }
}

fn draw_ultra_compact_footer(
    f: &mut Frame,
    app: &App,
    area: Rect,
    sub_color: ratatui::style::Color,
    main_color: ratatui::style::Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    if !app.current_quote_source.is_empty() {
        let source = Line::from(vec![
            Span::styled("source: ", Style::default().fg(sub_color)),
            Span::styled(&app.current_quote_source, Style::default().fg(main_color)),
        ]);
        f.render_widget(Paragraph::new(source).alignment(Alignment::Center), rows[0]);
    }
}

fn draw_chart(
    f: &mut Frame,
    app: &App,
    area: Rect,
    bg_color: ratatui::style::Color,
    sub_color:   ratatui::style::Color,
    main_color:  ratatui::style::Color,
    error_color: ratatui::style::Color,
    show_title: bool,
) {
    if app.wpm_history.is_empty() {
        f.render_widget(
            Paragraph::new("no data")
                .style(Style::default().fg(sub_color))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let rows = if show_title && area.height >= 10 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(area)
    };

    let (title_idx, chart_idx, legend_idx) = if show_title && area.height >= 10 {
        (Some(0), 1, 2)
    } else {
        (None, 0, 1)
    };

    if let Some(idx) = title_idx {
        let chart_title = Line::from(vec![
            Span::styled("Performance", Style::default().fg(sub_color)),
        ]);
        f.render_widget(Paragraph::new(chart_title).alignment(Alignment::Center), rows[idx]);
    }

    let chart_area = rows[chart_idx];
    let legend_area = rows[legend_idx];

    let filtered_wpm_history: Vec<(f64, f64)> = app.wpm_history.iter()
        .filter(|(t, _)| *t >= 1.0)
        .copied()
        .collect();

    let filtered_raw_wpm_history: Vec<(f64, f64)> = app.raw_wpm_history.iter()
        .filter(|(t, _)| *t >= 1.0)
        .copied()
        .collect();

    let max_time = filtered_wpm_history.iter()
        .map(|(t, _)| *t)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let max_wpm = filtered_wpm_history.iter().chain(filtered_raw_wpm_history.iter())
        .map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let y_max_wpm = (max_wpm * 1.2).max(10.0);

    let max_errors = app.errors_history.iter().map(|(_, e)| *e).fold(0.0_f64, f64::max);
    let y_max_err  = max_errors.max(1.0);

    let scaled_errors: Vec<(f64, f64)> = app.errors_history.iter()
        .filter(|(t, e)| *t >= 1.0 && *e > 0.0)
        .map(|(t, e)| (*t, (e / y_max_err) * y_max_wpm))
        .collect();

    const LEFT_W:  u16 = 5;
    const RIGHT_W: u16 = 6;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(LEFT_W),
            Constraint::Fill(1),
            Constraint::Length(RIGHT_W),
        ])
        .split(chart_area);

    let left_area  = cols[0];
    let body_area  = cols[1];
    let right_area = cols[2];

    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(sub_color))
            .data(&filtered_raw_wpm_history),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(main_color).add_modifier(ratatui::style::Modifier::BOLD))
            .data(&filtered_wpm_history),
        Dataset::default()
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(error_color))
            .data(&scaled_errors),
    ];

    let max_time_int = max_time.floor() as usize;
    let x_labels: Vec<Span> = (1..=max_time_int)
        .map(|t| {
            Span::styled(format!("{}", t), Style::default().fg(sub_color))
        })
        .collect();

    let blank_y: Vec<Span> = vec![
        Span::raw(""),
        Span::raw(""),
        Span::raw(""),
    ];

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(bg_color))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(sub_color))
                .bounds([1.0, max_time])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(sub_color))
                .bounds([0.0, y_max_wpm])
                .labels(blank_y),
        );

    f.render_widget(chart, body_area);

    let plot_top    = body_area.y;
    let plot_bottom = body_area.y + body_area.height.saturating_sub(2);
    let plot_height = plot_bottom.saturating_sub(plot_top);

    let label_rows: [(u16, f64, f64); 3] = [
        (plot_top,                     y_max_wpm,       y_max_err      ),
        (plot_top + plot_height / 2,   y_max_wpm / 2.0, y_max_err / 2.0),
        (plot_bottom,                  0.0,             0.0            ),
    ];

    for (idx, (row_y, wpm_val, err_val)) in label_rows.iter().enumerate() {
        if *row_y >= chart_area.y + chart_area.height { continue; }

        let wpm_text = format!("{:.0}", wpm_val);
        let wpm_rect = Rect::new(left_area.x, *row_y, LEFT_W, 1);
        f.render_widget(
            Paragraph::new(wpm_text)
                .style(Style::default().fg(main_color))
                .alignment(Alignment::Right),
            wpm_rect,
        );

        let err_text = format!("{:.0}", err_val);
        let skip_err = idx == 1 && err_text == "0" && format!("{:.0}", label_rows[2].2) == "0";

        if !skip_err {
            let err_rect = Rect::new(right_area.x, *row_y, RIGHT_W, 1);
            f.render_widget(
                Paragraph::new(err_text)
                    .style(Style::default().fg(error_color))
                    .alignment(Alignment::Left),
                err_rect,
            );
        }
    }

    if plot_height >= 6 {
        let wpm_title_y = plot_top + plot_height / 2;
        if wpm_title_y < chart_area.y + chart_area.height {
            f.render_widget(
                Paragraph::new("wpm")
                    .style(Style::default().fg(sub_color))
                    .alignment(Alignment::Right),
                Rect::new(chart_area.x, wpm_title_y.saturating_sub(1), LEFT_W, 1),
            );
        }

        let err_title_y = plot_top + plot_height / 2;
        if err_title_y < chart_area.y + chart_area.height {
            f.render_widget(
                Paragraph::new("err")
                    .style(Style::default().fg(sub_color))
                    .alignment(Alignment::Left),
                Rect::new(right_area.x, err_title_y.saturating_sub(1), RIGHT_W, 1),
            );
        }
    }

    let legend = Line::from(vec![
        Span::styled("━━ ", Style::default().fg(main_color).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("wpm  ", Style::default().fg(sub_color)),
        Span::styled("── ", Style::default().fg(sub_color)),
        Span::styled("raw  ", Style::default().fg(sub_color)),
        Span::styled("· ", Style::default().fg(error_color)),
        Span::styled("errors/s", Style::default().fg(sub_color)),
    ]);
    f.render_widget(
        Paragraph::new(legend).alignment(Alignment::Center),
        legend_area,
    );
}
