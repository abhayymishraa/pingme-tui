use anyhow::Result;
use app::{App, InputMode, LogEntry, LogLevel};
use clap::{Arg, Command};
use crossterm::{event, execute, terminal::enable_raw_mode};
use ratatui::prelude::CrosstermBackend;

use anyhow;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::{fs, io, path::Path, time::Duration as StdDuration};
use tokio::sync::mpsc;
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

mod app;
mod config;
mod ping;
mod storage;
mod ui;
mod visitor;

use config::Config;
use ping::{Endpoint, PingManager, PingResult};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run_app().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

async fn run_app() -> Result<()> {
    let matches = Command::new("pingme")
        .about("Monitor server uptime with enhanced visualization")
        .arg(Arg::new("url").help("URL to monitor").index(1))
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .value_name("FILE")
                .help("Configuration file"),
        )
        .get_matches();

    let mut ping_manager: PingManager = PingManager::new(60);

    if let Some(config_path) = matches.get_one::<String>("config") {
        if Path::new(config_path).exists() {
            let config_content = fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&config_content)?;

            for url in config.endpoints {
                ping_manager.add_endpoint(url)?;
            }
        }
    } else if let Some(url) = matches.get_one::<String>("url") {
        ping_manager.add_endpoint(url.clone())?;
    } else if Path::new(".ping").exists() {
        let config_content = fs::read_to_string(".ping")?;
        let config: Config = toml::from_str(&config_content)?;

        for url in config.endpoints {
            ping_manager.add_endpoint(url)?;
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (result_sender, mut result_receiver) = mpsc::unbounded_channel::<PingResult>();
    let (log_sender, mut log_receiver) = mpsc::unbounded_channel::<LogEntry>();

    let polling_manager = ping_manager.clone();
    tokio::spawn(async move {
        polling_manager
            .start_polling(result_sender, log_sender)
            .await;
    });

    let mut app = App::new();
    let storage = ping_manager.get_storage();

    app.add_log(LogLevel::Info, "Application started".to_string());

    loop {
        while let Ok(result) = result_receiver.try_recv() {
            if let Err(e) = storage.save_result(&result) {
                eprintln!("Error saving result: {}", e);
            }

            app.add_realtime_block(result.endpoint_id, result.status);

            if let Err(e) = app.update_stats(&storage) {
                eprintln!("Error updating status {}", e)
            }
        }

        while let Ok(log_entry) = log_receiver.try_recv() {
            app.logs.push(log_entry);
            if app.logs.len() > 1000 {
                app.logs.drain(0..100);
            }
        }

        app.update_stats(&storage)?;

        terminal.draw(|f| ui::ui(f, &app))?;

        if event::poll(StdDuration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => {
                            if app.developer_mode {
                                handle_developer_mode_input(&mut app, key.code);
                            } else {
                                if handle_normal_mode_input(&mut app, &storage, key.code)? {
                                    break;
                                }
                            }
                        }
                        InputMode::Adding => {
                            if handle_adding_mode_input(&mut app, &storage, key, &Event::Key(key))?
                            {
                            }
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_normal_mode_input(
    app: &mut App,
    storage: &crate::visitor::StorageVisitor,
    key_code: KeyCode,
) -> Result<bool> {
    match key_code {
        KeyCode::Char('q') => {
            app.add_log(LogLevel::Info, "Exiting application".to_string());
            return Ok(true);
        }
        KeyCode::Char('a') => {
            app.input_mode = InputMode::Adding;
            app.url_input = TextArea::default();
            app.add_log(LogLevel::Info, "Entering URL input mode".to_string());
        }
        KeyCode::Char('d') => {
            app.toggle_developer_mode();
            app.add_log(LogLevel::Info, "Switched to developer mode".to_string());
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_endpoint();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous_endpoint();
        }
        KeyCode::Char('r') => {
            app.add_log(LogLevel::Info, "Refreshing data...".to_string());
            if let Err(e) = app.update_stats(storage) {
                app.add_log(LogLevel::Error, format!("Failed to refresh: {}", e));
            } else {
                app.add_log(LogLevel::Success, "Data refreshed successfully".to_string());
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_developer_mode_input(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('q') => {
            app.toggle_developer_mode();
            app.add_log(LogLevel::Info, "Switched to normal mode".to_string());
        }
        KeyCode::Char('d') => {
            app.toggle_developer_mode();
            app.add_log(LogLevel::Info, "Switched to normal mode".to_string());
        }
        KeyCode::Up => app.scroll_logs_up(),
        KeyCode::Down => app.scroll_logs_down(),
        KeyCode::Char('c') => {
            app.logs.clear();
            app.add_log(LogLevel::Info, "Logs cleared".to_string());
        }
        _ => {}
    }
}

fn handle_adding_mode_input(
    app: &mut App,
    storage: &crate::visitor::StorageVisitor,
    key: crossterm::event::KeyEvent,
    event: &Event,
) -> Result<bool> {
    match key.code {
        KeyCode::Enter => {
            let url = app.url_input.lines().join("");
            if !url.is_empty() {
                let endpoint = Endpoint {
                    id: Uuid::new_v4(),
                    url: url.clone(),
                };
                match storage.add_endpoint(&endpoint) {
                    Ok(_) => {
                        app.add_log(LogLevel::Success, format!("Added endpoint: {}", url));
                    }
                    Err(e) => {
                        app.add_log(
                            LogLevel::Error,
                            format!("Failed to add endpoint {}: {}", url, e),
                        );
                    }
                }
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.add_log(LogLevel::Info, "Cancelled URL input".to_string());
        }
        _ => {
            app.url_input.input(Input::from(event.clone()));
        }
    }
    Ok(false)
}
