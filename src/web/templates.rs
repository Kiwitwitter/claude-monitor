use crate::monitor::state::Stats;
use crate::parser::SessionData;

/// Format token count with K/M suffix
fn format_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Render the main index page
pub fn render_index(stats: &Stats, active_sessions: &[&SessionData]) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Claude Monitor</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0f172a;
            color: #e2e8f0;
            min-height: 100vh;
            padding: 2rem;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{
            font-size: 2rem;
            margin-bottom: 2rem;
            color: #f8fafc;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        h1::before {{
            content: '';
            display: inline-block;
            width: 12px;
            height: 12px;
            background: #22c55e;
            border-radius: 50%;
            animation: pulse 2s infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
        .budget-section {{
            background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%);
            border-radius: 16px;
            padding: 1.5rem;
            margin-bottom: 2rem;
            border: 1px solid #334155;
        }}
        .budget-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1rem;
        }}
        .budget-title {{
            font-size: 1.25rem;
            color: #f8fafc;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .budget-percentage {{
            font-size: 2.5rem;
            font-weight: 700;
            color: #818cf8;
        }}
        .progress-container {{
            background: #0f172a;
            border-radius: 8px;
            height: 24px;
            overflow: hidden;
            margin-bottom: 1rem;
        }}
        .progress-bar {{
            height: 100%;
            border-radius: 8px;
            transition: width 0.5s ease;
        }}
        .progress-bar.low {{ background: linear-gradient(90deg, #22c55e, #4ade80); }}
        .progress-bar.medium {{ background: linear-gradient(90deg, #facc15, #fde047); }}
        .progress-bar.high {{ background: linear-gradient(90deg, #f97316, #fb923c); }}
        .progress-bar.critical {{ background: linear-gradient(90deg, #ef4444, #f87171); }}
        .budget-stats {{
            display: flex;
            justify-content: space-between;
            flex-wrap: wrap;
            gap: 1rem;
        }}
        .budget-stat {{
            display: flex;
            flex-direction: column;
            gap: 0.25rem;
        }}
        .budget-stat-label {{
            font-size: 0.75rem;
            color: #94a3b8;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .budget-stat-value {{
            font-size: 1.25rem;
            font-weight: 600;
            color: #f8fafc;
        }}
        .budget-stat-value.remaining {{ color: #22c55e; }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }}
        .stat-card {{
            background: #1e293b;
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid #334155;
        }}
        .stat-label {{
            font-size: 0.875rem;
            color: #94a3b8;
            margin-bottom: 0.5rem;
        }}
        .stat-value {{
            font-size: 2rem;
            font-weight: 700;
            color: #f8fafc;
        }}
        .stat-value.highlight {{ color: #818cf8; }}
        .stat-value.green {{ color: #22c55e; }}
        .stat-value.yellow {{ color: #facc15; }}
        .section {{
            background: #1e293b;
            border-radius: 12px;
            padding: 1.5rem;
            margin-bottom: 1.5rem;
            border: 1px solid #334155;
        }}
        .section-title {{
            font-size: 1.25rem;
            margin-bottom: 1rem;
            color: #f8fafc;
        }}
        .session-list {{ list-style: none; }}
        .session-item {{
            padding: 0.75rem 1rem;
            background: #0f172a;
            border-radius: 8px;
            margin-bottom: 0.5rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .session-item:last-child {{ margin-bottom: 0; }}
        .session-info {{ display: flex; flex-direction: column; gap: 0.25rem; }}
        .session-project {{ font-weight: 500; color: #e2e8f0; }}
        .session-id {{ font-size: 0.75rem; color: #64748b; font-family: monospace; }}
        .session-stats {{
            display: flex;
            gap: 1rem;
            font-size: 0.875rem;
            color: #94a3b8;
        }}
        .badge {{
            font-size: 0.75rem;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            background: #4f46e5;
            color: white;
        }}
        .badge.agent {{ background: #7c3aed; }}
        .project-list {{ list-style: none; }}
        .project-item {{
            padding: 0.75rem 1rem;
            background: #0f172a;
            border-radius: 8px;
            margin-bottom: 0.5rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .project-path {{ font-family: monospace; color: #e2e8f0; }}
        .project-stats {{ display: flex; gap: 1.5rem; font-size: 0.875rem; color: #94a3b8; }}
        .empty {{ color: #64748b; font-style: italic; padding: 1rem; text-align: center; }}
        .refresh-btn {{
            background: #3b82f6;
            color: white;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.875rem;
        }}
        .refresh-btn:hover {{ background: #2563eb; }}
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Claude Monitor</h1>
            <button class="refresh-btn" hx-get="/api/refresh" hx-swap="none" hx-on::after-request="htmx.trigger('#budget-container', 'refresh'); htmx.trigger('#stats-container', 'refresh'); htmx.trigger('#sessions-container', 'refresh');">
                Refresh
            </button>
        </div>

        <div id="budget-container" hx-get="/partials/budget" hx-trigger="load, refresh, every 10s" hx-swap="innerHTML">
            {budget_html}
        </div>

        <div id="stats-container" hx-get="/partials/stats" hx-trigger="load, refresh, every 10s" hx-swap="innerHTML">
            {stats_html}
        </div>

        <div id="sessions-container" hx-get="/partials/sessions" hx-trigger="load, refresh, every 10s" hx-swap="innerHTML">
            {sessions_html}
        </div>

        <div class="section">
            <h2 class="section-title">Projects by Usage</h2>
            {projects_html}
        </div>
    </div>
</body>
</html>"#,
        budget_html = render_budget_partial(stats),
        stats_html = render_stats_partial(stats),
        sessions_html = render_sessions_partial(active_sessions),
        projects_html = render_projects_list(stats),
    )
}

/// Render budget section partial
pub fn render_budget_partial(stats: &Stats) -> String {
    let percentage = stats.budget.percentage;
    let progress_class = if percentage < 50.0 {
        "low"
    } else if percentage < 75.0 {
        "medium"
    } else if percentage < 90.0 {
        "high"
    } else {
        "critical"
    };

    format!(
        r#"<div class="budget-section">
    <div class="budget-header">
        <div class="budget-title">
            <span>5-Hour Rolling Budget</span>
        </div>
        <div class="budget-percentage">{percentage:.1}%</div>
    </div>
    <div class="progress-container">
        <div class="progress-bar {progress_class}" style="width: {percentage}%;"></div>
    </div>
    <div class="budget-stats">
        <div class="budget-stat">
            <span class="budget-stat-label">Used</span>
            <span class="budget-stat-value">{used}</span>
        </div>
        <div class="budget-stat">
            <span class="budget-stat-label">Limit</span>
            <span class="budget-stat-value">{limit}</span>
        </div>
        <div class="budget-stat">
            <span class="budget-stat-label">Remaining</span>
            <span class="budget-stat-value remaining">{remaining}</span>
        </div>
    </div>
</div>"#,
        percentage = percentage,
        progress_class = progress_class,
        used = format_tokens(stats.budget.used),
        limit = format_tokens(stats.budget.limit),
        remaining = format_tokens(stats.budget.remaining),
    )
}

/// Render stats cards partial
pub fn render_stats_partial(stats: &Stats) -> String {
    let lifetime_total = stats.total_usage.total();

    format!(
        r#"<div class="stats-grid">
    <div class="stat-card">
        <div class="stat-label">Lifetime Total Tokens</div>
        <div class="stat-value highlight">{lifetime_total}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Lifetime Input</div>
        <div class="stat-value">{input_tokens}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Lifetime Output</div>
        <div class="stat-value">{output_tokens}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Cache Created</div>
        <div class="stat-value">{cache_created}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Cache Read</div>
        <div class="stat-value">{cache_read}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Active Sessions</div>
        <div class="stat-value green">{active_sessions}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Active Agents</div>
        <div class="stat-value yellow">{active_agents}</div>
    </div>
    <div class="stat-card">
        <div class="stat-label">Total Messages</div>
        <div class="stat-value">{total_messages}</div>
    </div>
</div>"#,
        lifetime_total = format_tokens(lifetime_total),
        input_tokens = format_tokens(stats.total_usage.input_tokens),
        output_tokens = format_tokens(stats.total_usage.output_tokens),
        cache_created = format_tokens(stats.total_usage.cache_creation_input_tokens),
        cache_read = format_tokens(stats.total_usage.cache_read_input_tokens),
        active_sessions = stats.active_sessions,
        active_agents = stats.active_agents,
        total_messages = stats.total_messages,
    )
}

/// Render active sessions list partial
pub fn render_sessions_partial(sessions: &[&SessionData]) -> String {
    if sessions.is_empty() {
        return r#"<div class="section">
            <h2 class="section-title">Active Sessions</h2>
            <div class="empty">No active sessions</div>
        </div>"#
            .to_string();
    }

    let items: Vec<String> = sessions
        .iter()
        .map(|s| {
            let badge = if s.is_agent {
                r#"<span class="badge agent">Agent</span>"#
            } else {
                r#"<span class="badge">Session</span>"#
            };

            format!(
                r#"<li class="session-item">
                <div class="session-info">
                    <span class="session-project">{project}</span>
                    <span class="session-id">{session_id}</span>
                </div>
                <div class="session-stats">
                    <span>{messages} msgs</span>
                    <span>{tokens} tokens</span>
                    {badge}
                </div>
            </li>"#,
                project = s.project_path,
                session_id = &s.session_id[..8.min(s.session_id.len())],
                messages = s.message_count,
                tokens = format_tokens(s.usage.total()),
                badge = badge,
            )
        })
        .collect();

    format!(
        r#"<div class="section">
        <h2 class="section-title">Active Sessions ({count})</h2>
        <ul class="session-list">
            {items}
        </ul>
    </div>"#,
        count = sessions.len(),
        items = items.join("\n")
    )
}

/// Render projects list
fn render_projects_list(stats: &Stats) -> String {
    if stats.projects.is_empty() {
        return r#"<div class="empty">No projects found</div>"#.to_string();
    }

    let items: Vec<String> = stats
        .projects
        .iter()
        .map(|p| {
            format!(
                r#"<li class="project-item">
                <span class="project-path">{path}</span>
                <div class="project-stats">
                    <span>{sessions} sessions</span>
                    <span>{messages} msgs</span>
                    <span>{tokens} tokens</span>
                </div>
            </li>"#,
                path = p.path,
                sessions = p.session_count,
                messages = p.message_count,
                tokens = format_tokens(p.usage.total()),
            )
        })
        .collect();

    format!(
        r#"<ul class="project-list">{items}</ul>"#,
        items = items.join("\n")
    )
}
