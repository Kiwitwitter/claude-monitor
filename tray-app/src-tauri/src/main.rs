// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod parser;

use parser::{get_stats, Stats};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent,
};
use tauri_plugin_shell::ShellExt;

struct AppState {
    stats: Arc<Mutex<Stats>>,
}

fn format_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn make_progress_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn build_menu(app: &tauri::AppHandle, stats: &Stats) -> Menu<tauri::Wry> {
    let menu = Menu::new(app).unwrap();

    // Header
    let header = MenuItem::new(app, "Claude Monitor", false, None::<&str>).unwrap();
    menu.append(&header).unwrap();

    // Separator
    let sep1 = MenuItem::new(app, "─────────────────────", false, None::<&str>).unwrap();
    menu.append(&sep1).unwrap();

    // Budget section
    let budget_header = MenuItem::new(app, "⏱ 5h Rolling Budget", false, None::<&str>).unwrap();
    menu.append(&budget_header).unwrap();

    // Progress bar
    let progress = make_progress_bar(stats.budget.percentage, 15);
    let progress_text = format!("   {} {:.1}%", progress, stats.budget.percentage);
    let progress_item = MenuItem::new(app, &progress_text, false, None::<&str>).unwrap();
    menu.append(&progress_item).unwrap();

    // Used / Remaining
    let used_text = format!(
        "   Used: {} / {}",
        format_tokens(stats.budget.used),
        format_tokens(stats.budget.limit)
    );
    let used_item = MenuItem::new(app, &used_text, false, None::<&str>).unwrap();
    menu.append(&used_item).unwrap();

    let remaining_text = format!("   Remaining: {}", format_tokens(stats.budget.remaining));
    let remaining_item = MenuItem::new(app, &remaining_text, false, None::<&str>).unwrap();
    menu.append(&remaining_item).unwrap();

    // Reset time
    if let Some(mins) = stats.budget.reset_minutes {
        let reset_text = if mins >= 60 {
            format!("   Resets in: {}h {}m", mins / 60, mins % 60)
        } else {
            format!("   Resets in: {}m", mins)
        };
        let reset_item = MenuItem::new(app, &reset_text, false, None::<&str>).unwrap();
        menu.append(&reset_item).unwrap();
    }

    // Separator
    let sep2 = MenuItem::new(app, "─────────────────────", false, None::<&str>).unwrap();
    menu.append(&sep2).unwrap();

    // Active sessions
    let active_text = format!(
        "Active: {} sessions, {} agents",
        stats.active_sessions, stats.active_agents
    );
    let active = MenuItem::new(app, &active_text, false, None::<&str>).unwrap();
    menu.append(&active).unwrap();

    // Separator
    let sep3 = MenuItem::new(app, "─────────────────────", false, None::<&str>).unwrap();
    menu.append(&sep3).unwrap();

    // Total usage section
    let total_header = MenuItem::new(app, "📊 Total Usage (All Time)", false, None::<&str>).unwrap();
    menu.append(&total_header).unwrap();

    let input_item = MenuItem::new(
        app,
        format!("   Input: {}", format_tokens(stats.total_usage.input_tokens)),
        false,
        None::<&str>,
    )
    .unwrap();
    menu.append(&input_item).unwrap();

    let output_item = MenuItem::new(
        app,
        format!("   Output: {}", format_tokens(stats.total_usage.output_tokens)),
        false,
        None::<&str>,
    )
    .unwrap();
    menu.append(&output_item).unwrap();

    let cache_item = MenuItem::new(
        app,
        format!("   Cache: {}", format_tokens(stats.total_usage.cache_read_input_tokens)),
        false,
        None::<&str>,
    )
    .unwrap();
    menu.append(&cache_item).unwrap();

    // Separator
    let sep4 = MenuItem::new(app, "─────────────────────", false, None::<&str>).unwrap();
    menu.append(&sep4).unwrap();

    // Projects header
    let proj_header = MenuItem::new(app, "📁 Top Projects", false, None::<&str>).unwrap();
    menu.append(&proj_header).unwrap();

    // Top 3 projects
    for proj in stats.projects.iter().take(3) {
        let short_path = proj.path.split('/').last().unwrap_or(&proj.path);
        let proj_text = format!("   {} - {}", short_path, format_tokens(proj.usage.total()));
        let proj_item = MenuItem::new(app, &proj_text, false, None::<&str>).unwrap();
        menu.append(&proj_item).unwrap();
    }

    // Separator
    let sep5 = MenuItem::new(app, "─────────────────────", false, None::<&str>).unwrap();
    menu.append(&sep5).unwrap();

    // Open Dashboard
    let dashboard = MenuItem::with_id(app, "dashboard", "🌐 Open Dashboard...", true, None::<&str>).unwrap();
    menu.append(&dashboard).unwrap();

    // Quit
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
    menu.append(&quit).unwrap();

    menu
}

fn build_title(stats: &Stats) -> String {
    // Show budget percentage in title
    format!("{:.0}%", stats.budget.percentage)
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // Another instance tried to launch - we just ignore it
            // The existing instance stays running
        }))
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initial stats
            let stats = get_stats().unwrap_or_default();
            let stats_arc = Arc::new(Mutex::new(stats.clone()));

            // Build initial menu
            let menu = build_menu(app.handle(), &stats);

            // Create tray icon with dynamic title showing budget %
            let title = build_title(&stats);

            // Load tray icon (embedded at compile time)
            let icon_bytes = include_bytes!("../icons/tray@2x.png");
            let icon_image = image::load_from_memory(icon_bytes).expect("Failed to load tray icon");
            let rgba = icon_image.to_rgba8();
            let (width, height) = rgba.dimensions();
            let icon = tauri::image::Image::new_owned(rgba.into_raw(), width, height);

            let _tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .title(&title)
                .tooltip("Claude Monitor - Token Budget")
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "dashboard" => {
                            let _ = app.shell().open("http://localhost:3456", None::<tauri_plugin_shell::open::Program>);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Ok(new_stats) = get_stats() {
                            let app = tray.app_handle();
                            let menu = build_menu(app, &new_stats);
                            let _ = tray.set_menu(Some(menu));
                            let _ = tray.set_title(Some(&build_title(&new_stats)));
                        }
                    }
                })
                .build(app)?;

            // Store state
            app.manage(AppState { stats: stats_arc.clone() });

            // Auto-refresh every 30 seconds
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(30));
                    if let Ok(new_stats) = get_stats() {
                        if let Some(tray) = app_handle.tray_by_id("main") {
                            let menu = build_menu(&app_handle, &new_stats);
                            let _ = tray.set_menu(Some(menu));
                            let _ = tray.set_title(Some(&build_title(&new_stats)));
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        // Only prevent exit when windows are closed, not when quit is explicitly called
        if let RunEvent::ExitRequested { code, api, .. } = event {
            // If code is Some, it's an explicit exit request (from app.exit())
            // If code is None, it's from closing windows - prevent that
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
