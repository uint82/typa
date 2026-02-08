pub mod results;
pub mod test;
pub mod utils;

use crate::app::App;
use crate::models::AppState;
use crate::ui::utils::hex_to_rgb;
use ratatui::{
    style::Style,
    widgets::Block,
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let bg_color = hex_to_rgb(&app.theme.bg);
    f.render_widget(
        Block::default().style(Style::default().bg(bg_color)),
        f.area(),
    );

    if app.state == AppState::Finished {
        results::draw(f, app);
    } else {
        test::draw(f, app);
    }
}
