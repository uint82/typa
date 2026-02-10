use crate::app::App;
use crate::models::{Mode, QuoteSelector};
use crate::ui::utils::{hex_to_rgb, get_quote_length_category, render_header, render_footer};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph},
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
            Constraint::Length(12),
            Constraint::Fill(1),
        ])
        .split(main_area);

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(80),
            Constraint::Fill(1),
        ])
        .split(vertical_chunks[1]);

    let area = horizontal_layout[1];

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
        ])
        .split(area);

    let sub_color = hex_to_rgb(&app.theme.sub);
    let main_color = hex_to_rgb(&app.theme.main);

    let wpm_line = Line::from(vec![
        Span::styled("wpm: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_wpm),
            Style::default()
                .fg(main_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]);

    f.render_widget(
        Paragraph::new(wpm_line).alignment(Alignment::Center),
        inner_layout[0],
    );

    let stats_line = Line::from(vec![
        Span::styled("acc: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}%", app.final_accuracy),
            Style::default().fg(main_color),
        ),
        Span::styled(" | raw: ", Style::default().fg(sub_color)),
        Span::styled(
            format!("{:.0}", app.final_raw_wpm),
            Style::default().fg(main_color),
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
        Span::styled(
            format!("{:.1}s", app.final_time),
            Style::default().fg(main_color),
        ),
    ]);

    f.render_widget(
        Paragraph::new(chars_line).alignment(Alignment::Center),
        inner_layout[2],
    );

    let mode_str = match &app.mode {
        Mode::Time(t) => format!("time {}", t),
        Mode::Words(w) => format!("word {}", w),
        Mode::Quote(q) => match q {
            QuoteSelector::Id(_) => {
                let actual_length = get_quote_length_category(app.original_quote_length);
                format!("quote {}", actual_length)
            },
            QuoteSelector::Category(len) => {
                let len_str = format!("{:?}", len).to_lowercase();
                let actual_length = if len_str == "all" {
                    get_quote_length_category(app.original_quote_length)
                } else {
                    &len_str
                };
                format!("quote {}", actual_length)
            },
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

    render_footer(f, app);
}
