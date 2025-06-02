use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_endpoints_table(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new([
        Cell::from("URL").style(Style::default().fg(Color::Yellow)),
        Cell::from("Status").style(Style::default().fg(Color::Yellow)),
        Cell::from("Uptime %").style(Style::default().fg(Color::Yellow)),
        Cell::from("Avg Latency").style(Style::default().fg(Color::Yellow)),
        Cell::from("Last Ping").style(Style::default().fg(Color::Yellow)),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .endpoints_stats
        .iter()
        .enumerate()
        .map(|(i, stats)| {
            let status = match stats.last_status {
                Some(true) => "UP",
                Some(false) => "DOWN",
                None => "N/A",
            };
            let status_color = match stats.last_status {
                Some(true) => Color::Green,
                Some(false) => Color::Red,
                None => Color::Gray,
            };

            let uptime = format!("{:.1}%", stats.uptime_percentage);
            let latency = stats
                .avg_latency
                .map_or("N/A".to_string(), |l| format!("{}ms", l));
            let last_ping = stats
                .last_ping
                .map_or("Never".to_string(), |dt| dt.format("%H:%M:%S").to_string());

            let mut style = Style::default();
            if i == app.selected_endpoint {
                style = style.fg(Color::Yellow);
            }

            Row::new(vec![
                Cell::from(stats.endpoint.url.clone()).style(style),
                Cell::from(status).style(Style::default().fg(status_color)),
                Cell::from(uptime).style(style),
                Cell::from(latency).style(style),
                Cell::from(last_ping).style(style),
            ])
            .bottom_margin(1)
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(40),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Endpoints")
            .border_style(Style::default().fg(Color::White)),
    );

    f.render_stateful_widget(table, area, &mut app.table_state.clone());
}
