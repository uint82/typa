mod cache;
mod draw;
mod stats;
pub mod history;

pub use history::{clear_history, delete_record, load_history, record_test, TestRecord};

use crate::config::Theme;
use crate::ui::utils::hex_to_rgb;
use anyhow::Result;
use cache::{
    build_chart_data, build_col_width_cache, build_detail_cache, build_row_cache,
    ColWidthCache, ColumnLayout, DetailCache, RowCache, compute_columns,
};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use draw::{draw, Palette};
use ratatui::{backend::CrosstermBackend, Terminal};
use stats::{build_stat_sections, local_datetime, sections_total_lines, StatSection};
use std::cell::Cell;
use std::io;

#[derive(PartialEq)]
pub(crate) enum View {
    Stats,
    History,
    Detail,
    Help,
}

pub(crate) struct Canvas {
    pub(crate) records: Vec<TestRecord>,
    pub(crate) history_indices: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) scroll_offset: usize,
    should_quit: bool,
    #[allow(dead_code)]
    theme: Theme,
    pub(crate) terminal_width: u16,
    pub(crate) terminal_height: u16,
    pub(crate) view: View,
    // Cell lets draw_table_rows update this through &Canvas without needing &mut.
    pub(crate) rendered_rows: Cell<usize>,
    pub(crate) stat_sections: Vec<StatSection>,
    pub(crate) record_dates: Vec<(String, String)>,
    pub(crate) stats_wpm_data:   Vec<(f64, f64)>,
    pub(crate) stats_acc_scaled: Vec<(f64, f64)>,
    pub(crate) stats_y_max:      f64,
    // index into self.records for each trend point. how we pin the selected record on the chart.
    pub(crate) trend_record_indices: Vec<usize>,
    pub(crate) row_cache: Vec<RowCache>,
    pub(crate) cols:   ColumnLayout,
    pub(crate) cols_w: usize,
    col_width_cache: ColWidthCache,
    pub(crate) detail_cache: Option<DetailCache>,
    pub(crate) stats_scroll: usize,
    pub(crate) stats_content_lines: usize,
    pub(crate) palette: Palette,
    pending_g: bool,
    pub(crate) pending_delete: bool,
}

impl Canvas {
    fn new(theme: Theme) -> Result<Self> {
        let mut records = load_history()?;
        records.reverse(); // newest first. the whole ui assumes this order.

        let stat_sections      = build_stat_sections(&records);
        let stats_content_lines = sections_total_lines(&stat_sections);
        let record_dates: Vec<(String, String)> = records.iter()
            .map(|r| local_datetime(&r.timestamp))
            .collect();
        let (stats_wpm_data, stats_acc_scaled, stats_y_max,
             trend_record_indices) = build_chart_data(&records);

        let history_indices: Vec<usize> = records.iter().enumerate()
            .filter(|(_, r)| r.completed)
            .map(|(i, _)| i)
            .collect();
        let completed: Vec<TestRecord> = history_indices.iter()
            .map(|&i| records[i].clone())
            .collect();
        let row_cache       = build_row_cache(&completed);
        let col_width_cache = build_col_width_cache(&completed);
        // zero width so resize() is forced to compute real columns before the first draw.
        let cols   = compute_columns(0, &col_width_cache);
        let cols_w = 0usize;

        let palette = Palette {
            bg:   hex_to_rgb(&theme.bg),
            main: hex_to_rgb(&theme.main),
            sub:  hex_to_rgb(&theme.sub),
        };

        Ok(Self {
            records,
            history_indices,
            selected: 0,
            scroll_offset: 0,
            should_quit: false,
            theme,
            terminal_width: 80,
            terminal_height: 24,
            view: View::Stats,
            rendered_rows: Cell::new(0),
            stat_sections,
            record_dates,
            stats_wpm_data,
            stats_acc_scaled,
            stats_y_max,
            trend_record_indices,
            row_cache,
            cols,
            cols_w,
            col_width_cache,
            detail_cache: None,
            stats_scroll: 0,
            stats_content_lines,
            palette,
            pending_g: false,
            pending_delete: false,
        })
    }

    fn resize(&mut self, w: u16, h: u16) {
        self.terminal_width = w;
        self.terminal_height = h;
        // mirrors the Percentage(80) constraint in draw() to keep in sync.
        let new_cols_w = (w as usize * 80) / 100;
        if new_cols_w != self.cols_w {
            self.cols_w = new_cols_w;
            self.cols = compute_columns(new_cols_w, &self.col_width_cache);
        }

        // re-clamp after resize so: (a) selected is always in the visible window,
        // (b) scroll_offset never leaves blank rows at the bottom.
        let vis   = self.visible_rows().max(1);
        let total = self.history_indices.len();

        let max_offset = total.saturating_sub(vis);
        self.scroll_offset = self.scroll_offset.min(max_offset);

        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }

        if self.selected >= self.scroll_offset + vis {
            self.scroll_offset = self.selected.saturating_sub(vis - 1);
        }
    }

    fn switch_view(&mut self) {
        self.view = match self.view {
            View::Stats              => View::History,
            View::History            => View::Stats,
            View::Detail | View::Help => View::History,
        };
    }

    fn open_detail(&mut self) {
        if !self.history_indices.is_empty() {
            let real_idx = self.history_indices[self.selected];
            self.detail_cache = Some(build_detail_cache(
                &self.records, &self.record_dates, real_idx,
            ));
            self.view = View::Detail;
        }
    }

    fn close_detail(&mut self) {
        self.detail_cache = None;
        self.view = View::History;
    }

    pub(crate) fn content_height(&self) -> u16 {
        self.terminal_height.saturating_sub(4)
    }

    pub(crate) fn chart_height(&self) -> u16 {
        let ch = self.content_height();
        if ch >= 18 { 10 }
        else if ch >= 12 { 6 }
        else { 0 }
    }

    pub(crate) fn visible_rows(&self) -> usize {
        // rendered_rows is the ground truth set by draw_table_rows each frame.
        // before the first draw it's 0, so we fall back to layout arithmetic.
        let r = self.rendered_rows.get();
        if r > 0 {
            return r;
        }
        let ch      = self.content_height();
        let chart_h = self.chart_height();
        let hdr     = chart_h + if chart_h > 0 { 1 } else { 0 };
        ch.saturating_sub(hdr + 2) as usize
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.history_indices.len() {
            self.selected += 1;
            let vis = self.visible_rows().max(1);
            if self.selected >= self.scroll_offset + vis {
                self.scroll_offset += 1;
            }
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    fn stats_scroll_down(&mut self) {
        let viewport = self.content_height() as usize;
        let max = self.stats_content_lines.saturating_sub(viewport);
        if self.stats_scroll < max {
            self.stats_scroll += 1;
        }
    }

    fn stats_scroll_up(&mut self) {
        self.stats_scroll = self.stats_scroll.saturating_sub(1);
    }

    fn jump_to_top(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn jump_to_bottom(&mut self) {
        let last = self.history_indices.len().saturating_sub(1);
        self.selected = last;
        let vis = self.visible_rows().max(1);
        self.scroll_offset = last.saturating_sub(vis - 1);
    }

    fn half_page_down(&mut self) {
        let vis  = self.visible_rows().max(2);
        let half = vis / 2;
        let last = self.history_indices.len().saturating_sub(1);
        self.selected = (self.selected + half).min(last);
        let ideal_offset = self.selected.saturating_sub(vis / 2);
        let max_offset   = last.saturating_sub(vis - 1);
        self.scroll_offset = ideal_offset.min(max_offset);
    }

    fn half_page_up(&mut self) {
        let vis  = self.visible_rows().max(2);
        let half = vis / 2;
        self.selected = self.selected.saturating_sub(half);
        self.scroll_offset = self.selected.saturating_sub(vis / 2);
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn open_help(&mut self) {
        self.pending_g = false;
        self.view = View::Help;
    }

    fn close_help(&mut self) {
        self.view = View::History;
    }

    fn confirm_delete(&mut self) {
        let real_idx = self.history_indices[self.selected];

        if let Err(e) = delete_record(real_idx, self.records.len()) {
            let _ = e;
            return;
        }

        self.records.remove(real_idx);

        self.history_indices = self.records.iter().enumerate()
            .filter(|(_, r)| r.completed)
            .map(|(i, _)| i)
            .collect();

        if !self.history_indices.is_empty() {
            self.selected = self.selected.min(self.history_indices.len() - 1);
        } else {
            self.selected = 0;
        }

        let completed: Vec<TestRecord> = self.history_indices.iter()
            .map(|&i| self.records[i].clone())
            .collect();

        self.row_cache          = build_row_cache(&completed);
        self.col_width_cache    = build_col_width_cache(&completed);
        self.cols               = compute_columns(self.cols_w, &self.col_width_cache);
        self.record_dates       = self.records.iter()
            .map(|r| local_datetime(&r.timestamp))
            .collect();
        let (wpm, acc, ymax, trend) = build_chart_data(&self.records);
        self.stats_wpm_data         = wpm;
        self.stats_acc_scaled       = acc;
        self.stats_y_max            = ymax;
        self.trend_record_indices   = trend;
        self.stat_sections          = build_stat_sections(&self.records);
        self.stats_content_lines    = sections_total_lines(&self.stat_sections);
        self.detail_cache           = None;

        let vis        = self.visible_rows().max(1);
        let max_offset = self.history_indices.len().saturating_sub(vis);
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }
}

pub fn run(theme: Theme) -> Result<()> {
    let mut canvas = Canvas::new(theme)?;

    if canvas.records.is_empty() || canvas.history_indices.is_empty() {
        println!("\n  No history yet. Complete a test to start tracking your progress.\n");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    canvas.resize(size.width, size.height);

    let result = run_loop(&mut terminal, &mut canvas);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    canvas: &mut Canvas,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, canvas))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char('q')
                        | KeyCode::Char('\x1b')
                        | KeyCode::Esc => {
                            if canvas.pending_delete {
                                canvas.pending_delete = false;
                            } else if canvas.view == View::Help {
                                canvas.close_help();
                            } else if canvas.view == View::Detail {
                                canvas.close_detail();
                            } else {
                                canvas.quit();
                            }
                        }
                        KeyCode::Char('c')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            canvas.quit()
                        }
                        KeyCode::Char('y') if canvas.pending_delete => {
                            canvas.pending_delete = false;
                            canvas.confirm_delete();
                        }
                        _ if canvas.pending_delete => {
                            canvas.pending_delete = false;
                        }
                        KeyCode::Tab
                        | KeyCode::Char('1')
                        | KeyCode::Char('2') => canvas.switch_view(),
                        KeyCode::Enter if canvas.view == View::History => canvas.open_detail(),
                        KeyCode::Char('?') if canvas.view == View::History => canvas.open_help(),
                        KeyCode::Char('d') if canvas.view == View::History => {
                            canvas.pending_g      = false;
                            canvas.pending_delete = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => match canvas.view {
                            View::History => canvas.move_down(),
                            View::Stats   => canvas.stats_scroll_down(),
                            View::Detail | View::Help => {}
                        },
                        KeyCode::Up | KeyCode::Char('k') => match canvas.view {
                            View::History => canvas.move_up(),
                            View::Stats   => canvas.stats_scroll_up(),
                            View::Detail | View::Help => {}
                        },
                        KeyCode::Char('G') if canvas.view == View::History => {
                            canvas.pending_g = false;
                            canvas.jump_to_bottom();
                        }
                        KeyCode::Char('g') if canvas.view == View::History => {
                            if canvas.pending_g {
                                canvas.pending_g = false;
                                canvas.jump_to_top();
                            } else {
                                canvas.pending_g = true;
                            }
                        }
                        KeyCode::Char('d')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                            && canvas.view == View::History =>
                        {
                            canvas.pending_g = false;
                            canvas.half_page_down();
                        }
                        KeyCode::Char('u')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                            && canvas.view == View::History =>
                        {
                            canvas.pending_g = false;
                            canvas.half_page_up();
                        }
                        _ => { canvas.pending_g = false; }
                    }
                }
                Event::Resize(w, h) => canvas.resize(w, h),
                _ => {}
            }
        }

        if canvas.should_quit {
            return Ok(());
        }
    }
}
