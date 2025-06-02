mod developer_ui;
use crate::app::App;
use developer_ui::developer_ui;
use main_ui::main_ui;
use ratatui::Frame;
mod main_ui;

pub fn ui(f: &mut Frame, app: &App) {
    if app.developer_mode {
        developer_ui(f, app);
    } else {
        main_ui(f, app);
    }
}
