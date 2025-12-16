# Claude Monitor

Monitor your Claude Code usage with a web dashboard and macOS menu bar app.

## Features

- Real-time token usage tracking (input/output/cache)
- Active sessions and agents count
- Per-project usage statistics
- Auto-refresh dashboard (every 10 seconds)
- File watcher for live updates
- **macOS Menu Bar App** - See stats at a glance

## Installation

### Web Dashboard

```bash
# Clone the repository
git clone https://github.com/Kiwitwitter/claude-monitor.git
cd claude-monitor

# Build
cargo build --release

# The binary will be at ./target/release/claude-monitor
```

### Menu Bar App (macOS)

```bash
cd tray-app
cargo tauri build

# Install the app
cp -r src-tauri/target/release/bundle/macos/Claude\ Monitor.app /Applications/
```

Or download the DMG from the releases page.

## Usage

### Web Dashboard

```bash
# Start the monitor server
./target/release/claude-monitor start --port 3456 --foreground

# Or run with cargo
cargo run -- start --port 3456 --foreground
```

Then open http://localhost:3456 in your browser.

### Menu Bar App

Just launch "Claude Monitor" from Applications. It will show:
- Token usage in the menu bar (e.g., `57.7K↑ 22.2K↓`)
- Click to see detailed stats
- Auto-refreshes every 30 seconds

## Commands (Web Dashboard)

```bash
claude-monitor start [OPTIONS]    # Start the monitor server
  -p, --port <PORT>               # Port to listen on (default: 3456)
  -f, --foreground                # Run in foreground

claude-monitor stop               # Stop the monitor server
claude-monitor status             # Show current status
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Dashboard web interface |
| `GET /api/stats` | Token usage statistics (JSON) |
| `GET /api/sessions` | Active sessions list (JSON) |
| `GET /api/refresh` | Force refresh data |

## Data Sources

Claude Monitor reads data from Claude Code's local storage at `~/.claude/`:

- `projects/{path}/{session}.jsonl` - Session data with token usage
- `projects/{path}/agent-*.jsonl` - Agent session data

## Tech Stack

- **Web Backend**: Rust + Axum
- **Web Frontend**: HTMX + Vanilla CSS
- **Menu Bar App**: Tauri 2
- **File Watching**: notify crate

## Screenshots

### Menu Bar
The menu bar shows input/output token counts at a glance.

### Dashboard
A full web dashboard with detailed statistics and per-project breakdowns.

## License

MIT
