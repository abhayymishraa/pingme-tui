# PingMe - Server Uptime Monitor

A terminal-based server uptime monitoring tool built with Rust and Ratatui that uses the Visitor pattern for clean architecture.

## Features

- **Real-time monitoring**: Polls endpoints every 60 seconds with HTTP health checks
- **SQLite storage**: Persistent storage of ping results and statistics
- **Terminal UI**: Interactive interface with endpoint table and uptime graphs
- **Visitor pattern**: Clean separation of concerns (polling, storage, display)
- **Retry logic**: 3 retry attempts for failed requests with exponential backoff
- **Multiple input methods**: Command line arguments or TOML configuration files

## Installation

1. Clone this repository
2. Make sure you have Rust installed (https://rustup.rs/)
3. Build the project:

```bash
cargo build --release
```

## Usage

### Monitor a single URL

```bash
cargo run -- https://example.com
```

### Use a configuration file

Create a `pingme.toml` file (see example below) and run:

```bash
cargo run -- --config pingme.toml
```

### Use default configuration

Place a `.ping` file in the current directory:

```bash
cargo run
```

## Configuration File Format

Create a `pingme.toml` file with the following structure:

```toml
# List of endpoints to monitor
endpoints = [
    "https://httpbin.org/status/200",
    "https://jsonplaceholder.typicode.com/posts/1",
    "https://api.github.com",
    "https://www.google.com"
]

# Polling interval in seconds (optional, defaults to 60)
interval_seconds = 60
```

## Terminal UI Controls

- **↑/↓**: Navigate between endpoints in the table
- **a**: Add a new URL to monitor
- **q**: Quit the application
- **ESC**: Cancel URL input
- **Enter**: Confirm URL input

## Display Features

### Endpoint Table
- **URL**: The monitored endpoint
- **Status**: UP/DOWN/N/A status with color coding
- **Uptime %**: Percentage uptime over all recorded pings
- **Avg Latency**: Average response time in milliseconds
- **Last Ping**: Time of the most recent ping

### Uptime Graph
- **24-hour history**: Line graph showing uptime percentage over time
- **Real-time updates**: Graph updates as new ping results come in
- **Per-endpoint view**: Select different endpoints to view their individual graphs

## Architecture

The application follows the Visitor pattern with these main components:

### Visitors
- **PollingVisitor**: Handles HTTP requests and retry logic
- **StorageVisitor**: Manages SQLite database operations
- **DisplayVisitor**: Placeholder for display-specific logic

### Core Components
- **PingManager**: Orchestrates the polling process and manages endpoints
- **App**: TUI application state and user interface logic
- **Data Models**: `Endpoint`, `PingResult`, and `EndpointStats` structs

### Database Schema

```sql
-- Endpoints table
CREATE TABLE endpoints (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL UNIQUE
);

-- Ping results table
CREATE TABLE ping_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint_id TEXT NOT NULL,
    status BOOLEAN NOT NULL,
    latency_ms INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (endpoint_id) REFERENCES endpoints (id)
);
```

## Dependencies

- **ratatui**: Terminal user interface framework
- **tui-textarea**: Text input widget for TUI
- **tokio**: Async runtime
- **reqwest**: HTTP client
- **sqlx**: Async SQL toolkit with SQLite support
- **serde**: Serialization/deserialization
- **clap**: Command line argument parsing
- **chrono**: Date and time handling
- **uuid**: Unique identifier generation

## Error Handling

- Network failures are handled with 3 retry attempts
- Database errors are logged and the application continues running
- Invalid URLs are rejected during input validation
- Terminal resize events are handled gracefully

## Development

To run in development mode:

```bash
cargo run -- --config pingme.toml
```

To run tests:

```bash
cargo test
```

## License

This project is open source and available under the MIT License.