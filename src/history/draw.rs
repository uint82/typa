use super::cache::ColumnLayout;
use super::{Canvas, View};
use super::stats::StatSection;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, Paragraph},
    Frame,
};

#[derive(Copy, Clone)]
pub(crate) struct Palette {
    pub(crate) bg:   ratatui::style::Color,
    pub(crate) main: ratatui::style::Color,
    pub(crate) sub:  ratatui::style::Color,
}

enum VLine {
    SectionTitle { title: String, col_header: Option<String> },
    Divider,
    DataRow   { label: String, value: String, label_w: usize },
    Gap,
}

fn draw_scrollbar(f: &mut Frame, area: Rect, total: usize, scroll: usize, p: &Palette) {
    let viewport    = area.height as usize;
    let thumb_h     = ((viewport * viewport) / total).max(1) as u16;
    let scroll_range = total.saturating_sub(viewport).max(1);
    let track_range  = area.height.saturating_sub(thumb_h).max(1);
    let thumb_top    = area.y
        + (scroll as f64 / scroll_range as f64 * track_range as f64) as u16;
    let thumb_top    = thumb_top.min(area.y + area.height.saturating_sub(thumb_h));
    let bar_x        = area.x + area.width.saturating_sub(1);

    for dy in 0..thumb_h {
        f.render_widget(
            Paragraph::new("█").style(Style::default().fg(p.sub)),
            Rect::new(bar_x, thumb_top + dy, 1, 1),
        );
    }
}

pub(crate) fn draw(f: &mut Frame, canvas: &Canvas) {
    let p = canvas.palette;

    f.render_widget(
        Block::default().style(Style::default().bg(p.bg)),
        f.area(),
    );

    let h = f.area().height;
    let w = f.area().width;

    let header_area  = Rect::new(0, 0, w, 2);
    let tab_bar_area = Rect::new(0, 2, w, 1);
    let footer_area  = Rect::new(0, h.saturating_sub(1), w, 1);
    let main_area    = Rect::new(0, 3, w, h.saturating_sub(4));

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Percentage(80), Constraint::Fill(1)])
        .split(main_area);

    let content_area = horizontal[1];

    draw_header(f, header_area, &canvas.view, &p);
    draw_tab_bar(f, tab_bar_area, &canvas.view, &p);

    match canvas.view {
        View::Stats   => draw_stats(f, canvas, content_area, &p),
        View::History | View::Detail | View::Help => {
            draw_history(f, canvas, content_area, &p);
            if canvas.view == View::Detail {
                draw_detail_modal(f, canvas, f.area(), &p);
            }
            if canvas.view == View::Help {
                draw_help_modal(f, f.area(), &p);
            }
        }
    }

    draw_footer(f, footer_area, &canvas.view, canvas.pending_delete, &p);
}

fn draw_header(f: &mut Frame, area: Rect, view: &View, p: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let subtitle = match view {
        View::Stats => "  stats",
        View::History | View::Detail
        | View::Help => "  history",
    };
    let title = Line::from(vec![
        Span::styled("typa", Style::default().fg(p.main).add_modifier(Modifier::BOLD)),
        Span::styled(subtitle, Style::default().fg(p.sub)),
    ]);
    f.render_widget(Paragraph::new(title).alignment(Alignment::Center), rows[0]);

    let border = "─".repeat(area.width as usize);
    f.render_widget(
        Paragraph::new(border).style(Style::default().fg(p.sub)),
        rows[1],
    );
}

fn draw_tab_bar(f: &mut Frame, area: Rect, view: &View, p: &Palette) {
    let (stats_style, history_style) = match view {
        View::Stats => (
            Style::default().fg(p.main).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            Style::default().fg(p.sub),
        ),
        View::History | View::Detail | View::Help => (
            Style::default().fg(p.sub),
            Style::default().fg(p.main).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
    };

    let line = Line::from(vec![
        Span::styled("stats", stats_style),
        Span::styled("  │  ", Style::default().fg(p.sub)),
        Span::styled("history", history_style),
    ]);
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn draw_footer(f: &mut Frame, area: Rect, view: &View, pending_delete: bool, p: &Palette) {
    let key = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
    let lbl = Style::default().fg(p.sub).add_modifier(Modifier::DIM);
    let sep = Style::default().fg(p.sub).add_modifier(Modifier::DIM);
    let dot = Span::styled("  •  ", sep);

    if pending_delete {
        let warn = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
        let spans = vec![
            Span::styled("delete this record? ", lbl),
            Span::styled("y", warn),
            Span::styled(" confirm  ", lbl),
            Span::styled("any other key", warn),
            Span::styled(" cancel", lbl),
        ];
        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
            area,
        );
        return;
    }

    let mut spans = vec![
        Span::styled("tab",   key), Span::styled(" switch", lbl), dot.clone(),
        Span::styled("enter", key), Span::styled(" open",   lbl), dot.clone(),
        Span::styled("jk/↑↓", key), Span::styled(" move",   lbl), dot.clone(),
        Span::styled("q",     key), Span::styled(" quit",   lbl),
    ];

    if matches!(view, View::History | View::Help) {
        spans.push(dot.clone());
        spans.push(Span::styled("?",     key));
        spans.push(Span::styled(" help",  lbl));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

pub(crate) fn draw_stats(f: &mut Frame, canvas: &Canvas, area: Rect, p: &Palette) {
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Fill(1)])
        .split(area);

    let overflows = canvas.stats_content_lines > area.height as usize;
    draw_stat_sections(f, &canvas.stat_sections, canvas.stats_scroll, overflows, h[0], p);
    draw_stats_dashboard_chart(
        f, &canvas.stats_wpm_data, &canvas.stats_acc_scaled, canvas.stats_y_max,
        h[1], p,
    );
}

fn draw_stat_sections(
    f: &mut Frame,
    sections: &[StatSection],
    scroll: usize,
    overflows: bool,
    area: Rect,
    p: &Palette,
) {
    let mut lines: Vec<VLine> = Vec::new();
    for section in sections {
        let label_w = section.rows.iter()
            .filter(|(l, _)| !l.is_empty())
            .map(|(l, _)| l.len())
            .max()
            .unwrap_or(0) + 2;

        lines.push(VLine::SectionTitle { title: section.title.clone(), col_header: section.col_header.clone() });
        lines.push(VLine::Divider);
        for (label, value) in &section.rows {
            lines.push(VLine::DataRow { label: label.clone(), value: value.clone(), label_w });
        }
        lines.push(VLine::Gap);
    }

    let total = lines.len();

    // one column sacrificed to the scrollbar god when content overflows.
    let draw_w  = if overflows { area.width.saturating_sub(1) } else { area.width };
    let bottom  = area.y + area.height;

    let mut y = area.y;
    for line in lines.iter().skip(scroll) {
        if y >= bottom { break; }

        match line {
            VLine::SectionTitle { title, col_header } => {
                f.render_widget(
                    Paragraph::new(title.as_str())
                        .style(Style::default().fg(p.sub).add_modifier(Modifier::DIM)),
                    Rect::new(area.x, y, draw_w, 1),
                );
                if let Some(hdr) = col_header {
                    f.render_widget(
                        Paragraph::new(hdr.as_str())
                            .style(Style::default().fg(p.sub).add_modifier(Modifier::DIM))
                            .alignment(Alignment::Right),
                        Rect::new(area.x, y, draw_w, 1),
                    );
                }
            }
            VLine::Divider => {
                f.render_widget(
                    Paragraph::new("─".repeat(draw_w as usize))
                        .style(Style::default().fg(p.sub).add_modifier(Modifier::DIM)),
                    Rect::new(area.x, y, draw_w, 1),
                );
            }
            VLine::DataRow { label, value, label_w } => {
                let lw = *label_w as u16;
                f.render_widget(
                    Paragraph::new(label.as_str())
                        .style(Style::default().fg(p.sub)),
                    Rect::new(area.x, y, lw.min(draw_w), 1),
                );
                let vw = draw_w.saturating_sub(lw);
                if vw > 0 {
                    f.render_widget(
                        Paragraph::new(value.as_str())
                            .style(Style::default().fg(p.main).add_modifier(Modifier::BOLD))
                            .alignment(Alignment::Right),
                        Rect::new(area.x + lw, y, vw, 1),
                    );
                }
            }
            VLine::Gap => {}
        }
        y += 1;
    }

    if overflows && total > 0 {
        draw_scrollbar(f, area, total, scroll, p);
    }
}

/// accuracy is scaled onto the wpm axis so both lines share the same y range;
/// the legend explains the mapping.
fn draw_stats_dashboard_chart(
    f: &mut Frame,
    wpm_data: &[(f64, f64)],
    acc_scaled: &[(f64, f64)],
    y_max: f64,
    area: Rect,
    p: &Palette,
) {
    if area.height < 3 || wpm_data.len() < 2 {
        return;
    }

    let n = wpm_data.len();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])
        .split(area);

    let title_area  = rows[0];
    let body_area   = rows[1];
    let legend_area = rows[2];

    f.render_widget(
        Paragraph::new(Span::styled("wpm & accuracy", Style::default().fg(p.sub)))
            .alignment(Alignment::Center),
        title_area,
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(5), Constraint::Fill(1), Constraint::Length(7)])
        .split(body_area);

    let label_left  = cols[0];
    let chart_body  = cols[1];
    let label_right = cols[2];

    let ch = chart_body.height;
    if ch >= 2 {
        let top_y    = chart_body.y;
        let bottom_y = chart_body.y + ch.saturating_sub(2);
        let mid_y    = top_y + (bottom_y - top_y) / 2;

        for (y, wpm_val) in [(top_y, y_max), (mid_y, y_max / 2.0), (bottom_y, 0.0)] {
            if y < label_left.y + label_left.height {
                f.render_widget(
                    Paragraph::new(format!("{:.0}", wpm_val))
                        .style(Style::default().fg(p.main))
                        .alignment(Alignment::Right),
                    Rect::new(label_left.x, y, label_left.width, 1),
                );

                let acc_pct = if y_max > 0.0 { (wpm_val / y_max) * 100.0 } else { 0.0 };
                f.render_widget(
                    Paragraph::new(format!("{:.0}%", acc_pct))
                        .style(Style::default().fg(p.sub).add_modifier(Modifier::DIM))
                        .alignment(Alignment::Left),
                    Rect::new(label_right.x, y, label_right.width, 1),
                );
            }
        }
    }

    let x_labels: Vec<Span> = if n <= 2 {
        vec![
            Span::styled("1", Style::default().fg(p.sub)),
            Span::styled(format!("{}", n), Style::default().fg(p.sub)),
        ]
    } else {
        vec![
            Span::styled("1",              Style::default().fg(p.sub)),
            Span::styled(format!("{}", n / 2), Style::default().fg(p.sub)),
            Span::styled(format!("{}", n), Style::default().fg(p.sub)),
        ]
    };

    let chart = Chart::new(vec![
        // accuracy rendered first so wpm wins every overlap fight.
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.sub).add_modifier(Modifier::DIM))
            .data(acc_scaled),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.main).add_modifier(Modifier::BOLD))
            .data(&wpm_data),
    ])
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().bg(p.bg))
    .x_axis(
        Axis::default()
            .style(Style::default().fg(p.sub))
            .bounds([1.0, n as f64])
            .labels(x_labels),
    )
    .y_axis(
        Axis::default()
            .style(Style::default().fg(p.sub))
            .bounds([0.0, y_max])
            .labels(vec![Span::raw(""), Span::raw(""), Span::raw("")]),
    );

    f.render_widget(chart, chart_body);

    let legend = Line::from(vec![
        Span::styled("━━ ", Style::default().fg(p.main).add_modifier(Modifier::BOLD)),
        Span::styled("wpm  ", Style::default().fg(p.sub)),
        Span::styled("── ", Style::default().fg(p.sub).add_modifier(Modifier::DIM)),
        Span::styled("accuracy", Style::default().fg(p.sub)),
    ]);
    f.render_widget(
        Paragraph::new(legend).alignment(Alignment::Center),
        legend_area,
    );
}

pub(crate) fn draw_history(f: &mut Frame, canvas: &Canvas, area: Rect, p: &Palette) {
    let chart_h = canvas.chart_height();

    let mut constraints = vec![];
    if chart_h > 0 {
        constraints.push(Constraint::Length(chart_h));
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(3));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    if chart_h > 0 {
        draw_trend_chart(f, canvas, &canvas.stats_wpm_data, &canvas.trend_record_indices, chunks[0], p);
        draw_table(f, canvas, chunks[2], p);
    } else {
        draw_table(f, canvas, chunks[0], p);
    }
}

fn draw_trend_chart(
    f: &mut Frame,
    canvas: &Canvas,
    wpm_data: &[(f64, f64)],
    record_indices: &[usize],
    area: Rect,
    p: &Palette,
) {
    if wpm_data.len() < 2 {
        f.render_widget(
            Paragraph::new("complete 2 or more tests to see your wpm graph")
                .style(Style::default().fg(p.sub))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let n = wpm_data.len();

    enum HighlightState {
        InWindow(Vec<(f64, f64)>),
        Incomplete,
    }

    let highlight_state = match canvas.records.get(canvas.selected) {
        Option::None => HighlightState::Incomplete,
        Some(rec) if !rec.completed => HighlightState::Incomplete,
        Some(_) => {
            // wpm can be None even on a completed test. treat as incomplete for display purposes.
            record_indices.iter().enumerate()
                .find(|&(_, &ri)| ri == canvas.selected)
                .and_then(|(i, &ri)| {
                    canvas.records[ri].wpm.map(|w| vec![(i as f64 + 1.0, w)])
                })
            .map_or(HighlightState::Incomplete, HighlightState::InWindow)
        }
    };

    let max_wpm = wpm_data.iter().map(|(_, w)| *w).fold(0.0_f64, f64::max);
    let y_max   = (max_wpm * 1.2).max(10.0);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(5), Constraint::Fill(1)])
        .split(area);

    let body_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(cols[1]);

    let title_line = {
        let base = Span::styled("wpm graph", Style::default().fg(p.sub));
        match &highlight_state {
            HighlightState::Incomplete => Line::from(vec![
                base,
                Span::styled(
                    "  (selected test incomplete)",
                    Style::default().fg(p.sub).add_modifier(Modifier::DIM),
                ),
            ]),
            HighlightState::InWindow(_) => Line::from(vec![base]),
        }
    };
    f.render_widget(
        Paragraph::new(title_line).alignment(Alignment::Center),
        body_rows[0],
    );

    let label_area = cols[0];
    let chart_h    = body_rows[1].height;
    if chart_h >= 2 {
        let top_y    = body_rows[1].y;
        let bottom_y = body_rows[1].y + chart_h.saturating_sub(2);
        let mid_y    = top_y + (bottom_y - top_y) / 2;
        for (y, val) in [(top_y, y_max), (mid_y, y_max / 2.0), (bottom_y, 0.0)] {
            if y < label_area.y + label_area.height {
                f.render_widget(
                    Paragraph::new(format!("{:.0}", val))
                        .style(Style::default().fg(p.sub))
                        .alignment(Alignment::Right),
                    Rect::new(label_area.x, y, label_area.width, 1),
                );
            }
        }
    }

    let highlight_pts: Vec<(f64, f64)> = match &highlight_state {
        HighlightState::InWindow(pts) => pts.clone(),
        _ => vec![],
    };

    let mut datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.main))
            .data(wpm_data),
    ];
    if !highlight_pts.is_empty() {
        datasets.push(
            Dataset::default()
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(p.main).add_modifier(Modifier::BOLD))
                .data(&highlight_pts),
        );
    }
    let x_labels: Vec<Span> = if n <= 2 {
        vec![
            Span::styled("1", Style::default().fg(p.sub)),
            Span::styled(format!("{}", n), Style::default().fg(p.sub)),
        ]
    } else {
        vec![
            Span::styled("1", Style::default().fg(p.sub)),
            Span::styled(format!("{}", n / 2), Style::default().fg(p.sub)),
            Span::styled(format!("{}", n), Style::default().fg(p.sub)),
        ]
    };

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(p.bg))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(p.sub))
                .bounds([1.0, n as f64])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(p.sub))
                .bounds([0.0, y_max])
                .labels(vec![Span::raw(""), Span::raw(""), Span::raw("")]),
        );

    f.render_widget(chart, body_rows[1]);
}

fn draw_help_modal(f: &mut Frame, area: Rect, p: &Palette) {
    let key = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
    let lbl = Style::default().fg(p.sub);
    let dim = Style::default().fg(p.sub).add_modifier(Modifier::DIM);

    let nav_title = "navigation";
    let nav_rows = [
        ("j / ↓",   "move down"),
        ("k / ↑",   "move up"),
        ("ctrl+d",  "half page down"),
        ("ctrl+u",  "half page up"),
        ("gg",      "jump to top"),
        ("G",       "jump to bottom"),
    ];

    let act_title = "actions";
    let act_rows = [
        ("enter",   "open detail"),
        ("d",       "delete record"),
        ("tab",     "switch view"),
        ("?",       "toggle help"),
        ("q / esc", "quit / close"),
    ];

    let key_w: usize = 10;
    let val_w: usize = 18;
    let inner_w = (key_w + val_w + 3) as u16;
    let total_rows = 1 + (2 + nav_rows.len()) + 1 + (2 + act_rows.len());
    let inner_h = total_rows as u16;
    let modal_w = (inner_w + 4).min(area.width.saturating_sub(4));
    let modal_h = (inner_h + 4).min(area.height.saturating_sub(2));
    let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(modal_x, modal_y, modal_w, modal_h);

    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.main))
            .style(Style::default().bg(p.bg)),
        modal_area,
    );

    let inner = Rect::new(
        modal_area.x + 2,
        modal_area.y + 1,
        modal_area.width.saturating_sub(4),
        modal_area.height.saturating_sub(2),
    );

    let mut y = inner.y;

    // modal title
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("keybindings", Style::default().fg(p.main).add_modifier(Modifier::BOLD)),
        ])).alignment(Alignment::Center),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;

    let mut draw_section = |y: &mut u16, title: &str, rows: &[(&str, &str)]| {
        if *y >= inner.y + inner.height { return; }
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(title.to_string(), dim)])),
            Rect::new(inner.x, *y, inner.width, 1),
        );
        *y += 1;
        if *y >= inner.y + inner.height { return; }
        f.render_widget(
            Paragraph::new("─".repeat(inner.width as usize)).style(dim),
            Rect::new(inner.x, *y, inner.width, 1),
        );
        *y += 1;
        for (k, v) in rows {
            if *y >= inner.y + inner.height { return; }
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{:<w$}", k, w = key_w), key),
                    Span::styled(*v, lbl),
                ])),
                Rect::new(inner.x, *y, inner.width, 1),
            );
            *y += 1;
        }
    };

    draw_section(&mut y, nav_title, &nav_rows[..]);
    y += 1; // gap between sections
    draw_section(&mut y, act_title, &act_rows[..]);
}

fn draw_detail_modal(f: &mut Frame, canvas: &Canvas, area: Rect, p: &Palette) {
    let Some(cache) = &canvas.detail_cache else { return; };
    let test_num = cache.test_num;
    let date     = &cache.date;
    let fields   = &cache.fields;
    let label_w  = cache.label_w;
    let value_w  = cache.value_w;
    let inner_w = (label_w + value_w + 2).max(34) as u16;
    let inner_h = fields.len() as u16 + 2;

    let modal_w = (inner_w + 2).min(area.width.saturating_sub(4));
    let modal_h = (inner_h + 2).min(area.height.saturating_sub(2));
    let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(modal_x, modal_y, modal_w, modal_h);

    // clear nukes the cells behind the modal. otherwise chart ghosts bleed through.
    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.main))
            .style(Style::default().bg(p.bg)),
        modal_area,
    );

    let inner = Rect::new(
        modal_area.x + 1,
        modal_area.y + 1,
        modal_area.width.saturating_sub(2),
        modal_area.height.saturating_sub(2),
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("test #{}", test_num),
                Style::default().fg(p.main).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ·  {}", date),
                Style::default().fg(p.sub),
            ),
        ])).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    f.render_widget(
        Paragraph::new("─".repeat(inner.width as usize))
            .style(Style::default().fg(p.sub)),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let lw     = label_w as u16;
    let body_y = inner.y + 2;

    for (i, (label, value)) in fields.iter().enumerate() {
        let y = body_y + i as u16;
        if y >= inner.y + inner.height { break; }

        f.render_widget(
            Paragraph::new(format!("{:<w$}", label, w = label_w))
                .style(Style::default().fg(p.sub)),
            Rect::new(inner.x, y, lw.min(inner.width), 1),
        );
        if lw < inner.width {
            f.render_widget(
                Paragraph::new(value.as_str())
                    .style(Style::default().fg(p.main).add_modifier(Modifier::BOLD)),
                Rect::new(inner.x + lw, y, inner.width - lw, 1),
            );
        }
    }
}

fn draw_table_header(f: &mut Frame, area: Rect, cols: &ColumnLayout, p: &Palette) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // "#↑" not "#↓" - oldest = 1, newest = N. the arrow exists so nobody "fixes" the sort.
    let mut spans = vec![Span::styled(
        format!("{:<sw$}{:<nw$}{:<dw$}", " ", "#↑", "date",
            sw = cols.w_sel, nw = cols.w_num, dw = cols.w_date),
        Style::default().fg(p.sub),
    )];

    if cols.show_mode { spans.push(Span::styled(format!("{:<w$}", "mode",     w = cols.w_mode), Style::default().fg(p.sub))); }
    if cols.show_lang { spans.push(Span::styled(format!("{:<w$}", "language", w = cols.w_lang), Style::default().fg(p.sub))); }
    spans.push(Span::styled(format!("{:<w$}", "wpm",  w = cols.w_wpm), Style::default().fg(p.sub)));
    if cols.show_raw  { spans.push(Span::styled(format!("{:<w$}", "raw",  w = cols.w_raw),  Style::default().fg(p.sub))); }
    spans.push(Span::styled(format!("{:<w$}", "acc",  w = cols.w_acc), Style::default().fg(p.sub)));
    if cols.show_con  { spans.push(Span::styled(format!("{:<w$}", "con",  w = cols.w_con),  Style::default().fg(p.sub))); }
    if cols.show_time { spans.push(Span::styled(format!("{:<w$}", "time", w = cols.w_time), Style::default().fg(p.sub))); }
    if cols.show_char { spans.push(Span::styled(format!("{:<w$}", "char", w = cols.w_char), Style::default().fg(p.sub))); }
    spans.push(Span::styled(format!("{:<w$}", "done", w = cols.w_done), Style::default().fg(p.sub)));

    f.render_widget(Paragraph::new(Line::from(spans)), sections[0]);

    let divider = "─".repeat(area.width as usize);
    f.render_widget(
        Paragraph::new(divider).style(Style::default().fg(p.sub)),
        sections[1],
    );
}

fn draw_table_rows(
    f: &mut Frame,
    canvas: &Canvas,
    rows_area: Rect,
    cols: &ColumnLayout,
    p: &Palette,
) {
    let visible = rows_area.height as usize;

    // set here so Canvas::visible_rows() can read it back. the only source of truth.
    canvas.rendered_rows.set(visible);

    for (display_idx, (record_idx, record)) in canvas
        .records.iter().enumerate()
        .skip(canvas.scroll_offset)
        .take(visible)
        .enumerate()
    {
        let row_y    = rows_area.y + display_idx as u16;
        let row_area = Rect::new(rows_area.x, row_y, rows_area.width, 1);
        let is_sel   = record_idx == canvas.selected;
        let cursor   = if is_sel { ">" } else { " " };
        let fg       = if is_sel { p.main } else { p.sub };

        let (date, _) = &canvas.record_dates[record_idx];
        let row = &canvas.row_cache[record_idx];

        let mut spans = vec![Span::styled(
            // test_num lives in RowCache so we never format it twice. don't move this.
            format!("{:<sw$}{:<nw$}{:<dw$}", cursor, row.test_num, date,
                sw = cols.w_sel, nw = cols.w_num, dw = cols.w_date),
            Style::default().fg(fg),
        )];
        if cols.show_mode { spans.push(Span::styled(format!("{:<w$}", row.mode,          w = cols.w_mode), Style::default().fg(fg))); }
        if cols.show_lang { spans.push(Span::styled(format!("{:<w$}", record.language,   w = cols.w_lang), Style::default().fg(fg))); }
        spans.push(Span::styled(format!("{:<w$}", row.wpm,  w = cols.w_wpm), Style::default().fg(fg)));
        if cols.show_raw  { spans.push(Span::styled(format!("{:<w$}", row.raw,  w = cols.w_raw),  Style::default().fg(fg))); }
        spans.push(Span::styled(format!("{:<w$}", row.acc,  w = cols.w_acc), Style::default().fg(fg)));
        if cols.show_con  { spans.push(Span::styled(format!("{:<w$}", row.con,  w = cols.w_con),  Style::default().fg(fg))); }
        if cols.show_time { spans.push(Span::styled(format!("{:<w$}", row.time, w = cols.w_time), Style::default().fg(fg))); }
        if cols.show_char { spans.push(Span::styled(format!("{:<w$}", row.char_stats, w = cols.w_char), Style::default().fg(fg))); }
        spans.push(Span::styled(format!("{:<w$}", row.done, w = cols.w_done), Style::default().fg(fg)));

        f.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }
}

fn draw_table(f: &mut Frame, canvas: &Canvas, area: Rect, p: &Palette) {
    // cols is a plain field, not a RefCell, resize() owns the recompute, not draw().
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Fill(1),
        ])
        .split(area);

    draw_table_header(f, sections[0], &canvas.cols, p);
    draw_table_rows(f, canvas, sections[1], &canvas.cols, p);
}
