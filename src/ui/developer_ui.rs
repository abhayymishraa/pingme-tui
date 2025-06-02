use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, LogLevel};

pub fn developer_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(f.size());

    let header = Paragraph::new("Developer Logs (Press 'd' to go back to main view)")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Developer Mode")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(header, chunks[0]);

    // Logs
    let visible_logs = if app.logs.len() <= chunks[1].height as usize - 2 {
        app.logs.iter().collect::<Vec<_>>()
    } else {
        let start = app.log_scroll;
        let end = std::cmp::min(start + (chunks[1].height as usize - 2), app.logs.len());
        app.logs[start..end].iter().collect::<Vec<_>>()
    };

    let log_rows: Vec<Row> = visible_logs
        .iter()
        .map(|log| {
            let color = match log.level {
                LogLevel::Info => Color::White,
                LogLevel::Error => Color::Red,
                LogLevel::Success => Color::Green,
                LogLevel::Warning => Color::Yellow,
            };

            let level_str = match log.level {
                LogLevel::Info => "INFO",
                LogLevel::Error => "ERROR",
                LogLevel::Success => "SUCCESS",
                LogLevel::Warning => "WARN",
            };

            Row::new(vec![
                Cell::from(log.timestamp.format("%H:%M:%S").to_string()),
                Cell::from(level_str).style(Style::default().fg(color)),
                Cell::from(log.message.clone()),
            ])
        })
        .collect();

    let logs_table = Table::new(
        log_rows,
        &[
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Percentage(80),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("Time").style(Style::default().fg(Color::Yellow)),
            Cell::from("Level").style(Style::default().fg(Color::Yellow)),
            Cell::from("Message").style(Style::default().fg(Color::Yellow)),
        ])
        .height(1)
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Logs ({}/{})", app.logs.len(), 1000))
            .border_style(Style::default().fg(Color::White)),
    );

    f.render_widget(logs_table, chunks[1]);

    // Instructions
    let instructions = Paragraph::new("↑/↓: Scroll logs | d: Back to main | q: Quit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(instructions, chunks[2]);
}
