use crate::app::App;
use crate::models::{Mode, QuoteSelector};
use crate::ui::utils::hex_to_rgb;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
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
