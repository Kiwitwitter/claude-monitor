# Claude Monitor

A local web dashboard to monitor your Claude Code usage in real-time.

## Features

- Real-time token usage tracking (input/output/cache)
- Active sessions and agents count
- Per-project usage statistics
- Auto-refresh dashboard (every 10 seconds)
- File watcher for live updates

## Installation

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/claude-monitor.git
cd claude-monitor

# Build
cargo build --release

# The binary will be at ./target/release/claude-monitor
```

## Usage

```bash
# Start the monitor server
./target/release/claude-monitor start --port 3456 --foreground

# Or run with cargo
cargo run -- start --port 3456 --foreground
```

Then open http://localhost:3456 in your browser.

## Commands

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

- **Backend**: Rust + Axum
- **Frontend**: HTMX + Vanilla CSS
- **File Watching**: notify crate

## License

MIT
