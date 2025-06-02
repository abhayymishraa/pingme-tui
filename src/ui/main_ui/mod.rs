use endpoints_table::render_endpoints_table;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, UptimeBlock};

mod endpoints_table;

pub fn main_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(f.size());

    render_endpoints_table(f, app, chunks[0]);

    render_uptime_chart(f, app, chunks[1]);

    render_uptime_blocks(f, app, chunks[2]);

    render_input_section(f, app, chunks[3]);
}

fn render_uptime_chart(f: &mut Frame, app: &App, area: Rect) {
    if !app.endpoints_stats.is_empty() && app.selected_endpoint < app.endpoints_stats.len() {
        let selected_endpoint = &app.endpoints_stats[app.selected_endpoint];

        if let Some(data) = app
            .uptime_history
            .get(&selected_endpoint.endpoint.id)
            .cloned()
        {
            if !data.is_empty() {
                let time_range = app.get_current_time_range();
                let datasets = vec![Dataset::default()
                    .name(selected_endpoint.endpoint.url.clone())
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Cyan))
                    .graph_type(GraphType::Line)
                    .data(&data)];

                let chart = Chart::new(datasets)
                    .block(
                        Block::default()
                            .title(format!(
                                "{} Uptime History - {}",
                                time_range.display_name(),
                                selected_endpoint.endpoint.url
                            ))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::White)),
                    )
                    .x_axis(
                        Axis::default()
                            .title("Time")
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, time_range.get_duration_hours().unwrap_or(24) as f64])
                            .labels(generate_time_labels(time_range)),
                    )
                    .y_axis(
                        Axis::default()
                            .title("Uptime %")
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, 100.0])
                            .labels(vec![
                                "0%".into(),
                                "25%".into(),
                                "50%".into(),
                                "75%".into(),
                                "100%".into(),
                            ]),
                    );

                f.render_widget(chart, area);
            } else {
                let no_data = Paragraph::new("No uptime data available yet...")
                    .block(
                        Block::default()
                            .title("Uptime History")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow)),
                    )
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(no_data, area);
            }
        }
    }
}

fn render_uptime_blocks(f: &mut Frame, app: &App, area: Rect) {
    if !app.endpoints_stats.is_empty() && app.selected_endpoint < app.endpoints_stats.len() {
        let selected_endpoint = &app.endpoints_stats[app.selected_endpoint];
        let time_range = app.get_current_time_range();

        if let Some(blocks) = app.uptime_blocks.get(&selected_endpoint.endpoint.id) {
            let title = format!(
                "Uptime Status - {} ({})",
                selected_endpoint.endpoint.url,
                time_range.display_name()
            );

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));

            f.render_widget(block, area);

            let inner_area = area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            });

            render_status_blocks(f, blocks, inner_area, time_range);
        } else {
            let no_data = Paragraph::new("No uptime status data available...")
                .block(
                    Block::default()
                        .title("Uptime Status")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().fg(Color::Gray));
            f.render_widget(no_data, area);
        }
    }
}

fn render_status_blocks(
    f: &mut Frame,
    blocks: &[UptimeBlock],
    area: Rect,
    time_range: &crate::app::TimeRange,
) {
    if blocks.is_empty() || area.height < 3 {
        return;
    }

    let available_width = area.width.saturating_sub(2) as usize;
    let available_height = area.height.saturating_sub(2) as usize;

    let (block_width, block_height, blocks_per_row) =
        calculate_block_dimensions(available_width, time_range);

    let mut lines = Vec::new();

    for row in 0..(blocks.len() + blocks_per_row - 1) / blocks_per_row {
        let mut spans = Vec::new();

        for col in 0..blocks_per_row {
            let block_index = row * blocks_per_row + col;

            if block_index < blocks.len() {
                let block = &blocks[block_index];
                let color = get_uptime_color(block.status);
                let block_char = get_block_char(block_width);

                spans.push(Span::styled(
                    block_char.repeat(block_width),
                    Style::default().fg(color),
                ));
                spans.push(Span::raw(" "));
            } else {
                spans.push(Span::raw(" ".repeat(block_width + 1)));
            }
        }

        for _ in 0..block_height {
            lines.push(Line::from(spans.clone()));
        }
    }

    if lines.len() < available_height {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("█", Style::default().fg(Color::Green)),
            Span::raw(" Up  "),
            Span::styled("█", Style::default().fg(Color::Red)),
            Span::raw(" Down  "),
        ]));
    }

    let status_paragraph = Paragraph::new(lines);
    f.render_widget(status_paragraph, area);
}

fn calculate_block_dimensions(
    available_width: usize,
    time_range: &crate::app::TimeRange,
) -> (usize, usize, usize) {
    match time_range {
        crate::app::TimeRange::Minutes(60) => {
            let block_width = 1;
            let block_height = 1;
            let blocks_per_row = available_width / (block_width + 1);
            (block_width, block_height, blocks_per_row.max(1))
        }
        _ => {
            let block_width = 1;
            let block_height = 1;
            let blocks_per_row = available_width / (block_width + 1);
            (block_width, block_height, blocks_per_row.max(1))
        }
    }
}

fn get_uptime_color(status: bool) -> Color {
    if status {
        Color::Green
    } else {
        Color::Red
    }
}

fn get_block_char(width: usize) -> &'static str {
    match width {
        1 => "█",
        _ => "█",
    }
}

fn generate_time_labels(time_range: &crate::app::TimeRange) -> Vec<ratatui::text::Span<'static>> {
    match time_range {
        crate::app::TimeRange::Minutes(m) => {
            vec![
                format!("{}m ago", m).into(),
                format!("{}m ago", m * 3 / 4).into(),
                format!("{}m ago", m / 2).into(),
                format!("{}m ago", m / 4).into(),
                "Now".into(),
            ]
        }
    }
}

fn render_input_section(f: &mut Frame, app: &App, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(match app.input_mode {
            InputMode::Normal => "Press 'a' to add URL, 'q' to quit",
            InputMode::Adding => "Enter URL (ESC to cancel, Enter to confirm)",
        });
    f.render_widget(input_block, area);

    if app.input_mode == InputMode::Adding {
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });
        let text = app.url_input.lines().join("\n");
        f.render_widget(Paragraph::new(text), inner_area);
    }
}
