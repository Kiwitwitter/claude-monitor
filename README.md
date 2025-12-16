# Claude Monitor

A real-time monitoring tool for [Claude Code](https://claude.ai/code) usage. Track your token consumption, active sessions, and budget with a web dashboard and native macOS menu bar app.

## Features

### Budget Tracking
- **5-Hour Rolling Window**: Monitor your token budget based on Claude's Max plan limits
- **Visual Progress Bar**: Color-coded usage indicator (green/yellow/orange/red)

### Token Analytics
- **Lifetime Statistics**: Total tokens used across all sessions
- **Real-time Tracking**: Input, output, and cache token breakdown
- **Per-Project Breakdown**: See which projects consume the most tokens

### Session Monitoring
- **Active Sessions**: Track currently running Claude Code sessions
- **Agent Detection**: Identify active autonomous agents
- **Message Counts**: Total messages per session and project

### Dual Interface
- **Web Dashboard**: Full-featured browser interface with auto-refresh
- **macOS Menu Bar App**: Quick glance at stats without leaving your workflow

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Cargo](https://doc.rust-lang.org/cargo/)
- For menu bar app: macOS 10.15+ and [Tauri CLI](https://tauri.app/)

## Installation

### Web Dashboard

```bash
# Clone the repository
git clone https://github.com/Kiwitwitter/claude-monitor.git
cd claude-monitor

# Build release binary
cargo build --release

# Binary location: ./target/release/claude-monitor
```

### Menu Bar App (macOS)

#### Option 1: Build from Source

```bash
# Install Tauri CLI if not already installed
cargo install tauri-cli

# Build the app
cd tray-app
cargo tauri build

# Install to Applications
cp -r src-tauri/target/release/bundle/macos/Claude\ Monitor.app /Applications/
```

#### Option 2: Install from DMG

After building, a DMG file is created at:
```
tray-app/src-tauri/target/release/bundle/dmg/Claude Monitor_x.x.x_aarch64.dmg
```

Double-click the DMG and drag "Claude Monitor" to your Applications folder.

#### First Launch on macOS

When launching for the first time, macOS may show a security warning:

1. **"Claude Monitor" cannot be opened because the developer cannot be verified**
   - Go to **System Preferences > Privacy & Security**
   - Scroll down and click **"Open Anyway"**
   - Or right-click the app and select **"Open"**

2. **Grant necessary permissions if prompted**
   - The app only reads files from `~/.claude/` directory
   - No network access required (except to open localhost dashboard)

## Usage

### Web Dashboard

```bash
# Start the server (default port: 3456)
./target/release/claude-monitor

# Or with cargo
cargo run
```

Open http://localhost:3456 in your browser.

**Dashboard Features:**
- Budget progress bar with percentage and remaining tokens
- Lifetime token statistics
- Active sessions list
- Per-project usage breakdown
- Auto-refresh every 10 seconds

### Menu Bar App

#### Launching

```bash
# From terminal
open /Applications/Claude\ Monitor.app

# Or use Spotlight (Cmd + Space)
# Type "Claude Monitor" and press Enter
```

#### What You'll See

1. **Menu Bar Icon**: A robot icon appears in your menu bar with the current budget percentage (e.g., "2%")

2. **Click the Icon** to see:
   - 5-hour rolling budget progress bar
   - Used/Limit/Remaining tokens
   - Active sessions and agents count
   - Lifetime token usage (input/output/cache)
   - Top 3 projects by usage
   - "Open Dashboard" link
   - "Quit" option

#### Controls

| Action | Result |
|--------|--------|
| Click icon | Refresh data and show menu |
| "Open Dashboard" | Opens web dashboard in browser |
| "Quit" | Exit the app |

**Menu Bar Features:**
- Shows budget usage percentage in the menu bar
- Click icon to refresh and view detailed stats
- 5-hour rolling budget with progress visualization
- Used/remaining tokens
- Lifetime token breakdown (input/output/cache)
- Top projects by usage
- Quick link to open web dashboard
- Auto-refresh every 30 seconds
- Single instance (won't duplicate if opened multiple times)

#### Launch at Login (Optional)

To start Claude Monitor automatically when you log in:

1. Open **System Preferences > General > Login Items**
2. Click **+** and select **Claude Monitor** from Applications
3. Or drag the app to the Login Items list

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Web dashboard interface |
| `GET /api/stats` | Token usage statistics (JSON) |
| `GET /api/sessions` | Active sessions list (JSON) |
| `GET /api/refresh` | Force data refresh |
| `GET /partials/budget` | Budget section (HTMX partial) |
| `GET /partials/stats` | Stats cards (HTMX partial) |
| `GET /partials/sessions` | Sessions list (HTMX partial) |

### Example API Response

```json
{
  "total_usage": {
    "input_tokens": 60086,
    "output_tokens": 65474,
    "cache_creation_input_tokens": 1019235,
    "cache_read_input_tokens": 26032347
  },
  "budget": {
    "limit": 45000000,
    "used": 1122399,
    "remaining": 43877601,
    "percentage": 2.49,
    "window_hours": 5
  },
  "active_sessions": 1,
  "active_agents": 0,
  "total_messages": 653
}
```

## Data Sources

Claude Monitor reads data from Claude Code's local storage:

```
~/.claude/
└── projects/
    └── {encoded-path}/
        ├── {session-id}.jsonl      # Regular sessions
        └── agent-{session-id}.jsonl # Agent sessions
```

Each JSONL file contains message entries with token usage:

```json
{
  "type": "assistant",
  "message": {
    "usage": {
      "input_tokens": 100,
      "output_tokens": 50,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 500
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Web Backend | Rust + Axum |
| Web Frontend | HTMX + CSS |
| Menu Bar App | Tauri 2 |
| File Watching | notify crate |
| Date/Time | chrono |
| Serialization | serde + serde_json |

## Project Structure

```
claude-monitor/
├── src/
│   ├── main.rs           # Entry point, CLI, server
│   ├── config.rs         # Configuration
│   ├── parser/           # JSONL parsing
│   │   ├── session.rs    # Session data & budget
│   │   └── history.rs    # History parsing
│   ├── monitor/          # State management
│   │   ├── state.rs      # App state & stats
│   │   └── watcher.rs    # File system watcher
│   ├── api/
│   │   └── routes.rs     # HTTP routes
│   └── web/
│       └── templates.rs  # HTML templates
├── tray-app/             # macOS menu bar app
│   └── src-tauri/
│       ├── src/
│       │   ├── main.rs   # Tauri app
│       │   └── parser.rs # Shared parsing logic
│       └── icons/        # App icons
└── Cargo.toml
```

## Configuration

Default settings:
- **Web Server Port**: 3456
- **Auto-refresh (Web)**: 10 seconds
- **Auto-refresh (Menu Bar)**: 30 seconds
- **Active Session Threshold**: 5 minutes since last activity
- **Rolling Budget Window**: 5 hours
- **Default Token Limit**: 45,000,000 (Max plan)

## Troubleshooting

### Web dashboard shows no data
- Ensure Claude Code has been used and created session files
- Check that `~/.claude/projects/` directory exists
- Verify file permissions

### Menu bar app shows two icons
- This was fixed in recent versions
- Kill all instances: `pkill -f "Claude Monitor"`
- Relaunch the app

### Port already in use
- Another instance may be running
- Kill existing process: `pkill -f claude-monitor`
- Or use a different port: `cargo run -- --port 3457`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

GPL-3.0 License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Built for monitoring [Claude Code](https://claude.ai/code) by Anthropic
- Powered by [Tauri](https://tauri.app/) for the menu bar app
- Uses [Axum](https://github.com/tokio-rs/axum) for the web server
