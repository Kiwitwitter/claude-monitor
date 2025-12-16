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

fn build_menu(app: &tauri::AppHandle, stats: &Stats) -> Menu<tauri::Wry> {
    let input = format_tokens(stats.total_usage.input_tokens);
    let output = format_tokens(stats.total_usage.output_tokens);
    let cache_read = format_tokens(stats.total_usage.cache_read_input_tokens);

    let menu = Menu::new(app).unwrap();

    // Header
    let header = MenuItem::new(app, "Claude Monitor", false, None::<&str>).unwrap();
    menu.append(&header).unwrap();

    // Separator
    let sep1 = MenuItem::new(app, "─────────────────", false, None::<&str>).unwrap();
    menu.append(&sep1).unwrap();

    // Stats
    let active_text = format!(
        "Active: {} sessions, {} agents",
        stats.active_sessions, stats.active_agents
    );
    let active = MenuItem::new(app, &active_text, false, None::<&str>).unwrap();
    menu.append(&active).unwrap();

    let input_item = MenuItem::new(app, format!("Input: {} tokens", input), false, None::<&str>).unwrap();
    menu.append(&input_item).unwrap();

    let output_item = MenuItem::new(app, format!("Output: {} tokens", output), false, None::<&str>).unwrap();
    menu.append(&output_item).unwrap();

    let cache_item = MenuItem::new(app, format!("Cache Read: {} tokens", cache_read), false, None::<&str>).unwrap();
    menu.append(&cache_item).unwrap();

    let msgs_item = MenuItem::new(app, format!("Messages: {}", stats.total_messages), false, None::<&str>).unwrap();
    menu.append(&msgs_item).unwrap();

    // Separator
    let sep2 = MenuItem::new(app, "─────────────────", false, None::<&str>).unwrap();
    menu.append(&sep2).unwrap();

    // Projects header
    let proj_header = MenuItem::new(app, "Projects:", false, None::<&str>).unwrap();
    menu.append(&proj_header).unwrap();

    // Top 3 projects
    for proj in stats.projects.iter().take(3) {
        let short_path = proj.path.split('/').last().unwrap_or(&proj.path);
        let proj_text = format!("  {} - {}", short_path, format_tokens(proj.usage.total()));
        let proj_item = MenuItem::new(app, &proj_text, false, None::<&str>).unwrap();
        menu.append(&proj_item).unwrap();
    }

    // Separator
    let sep3 = MenuItem::new(app, "─────────────────", false, None::<&str>).unwrap();
    menu.append(&sep3).unwrap();

    // Refresh
    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>).unwrap();
    menu.append(&refresh).unwrap();

    // Open Dashboard
    let dashboard = MenuItem::with_id(app, "dashboard", "Open Dashboard...", true, None::<&str>).unwrap();
    menu.append(&dashboard).unwrap();

    // Quit
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
    menu.append(&quit).unwrap();

    menu
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initial stats
            let stats = get_stats().unwrap_or_default();
            let stats_arc = Arc::new(Mutex::new(stats.clone()));

            // Build initial menu
            let menu = build_menu(app.handle(), &stats);

            // Create tray icon with dynamic title
            let title = format!(
                "{}↑ {}↓",
                format_tokens(stats.total_usage.input_tokens),
                format_tokens(stats.total_usage.output_tokens)
            );

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .title(&title)
                .tooltip("Claude Monitor")
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "refresh" => {
                            if let Ok(new_stats) = get_stats() {
                                // Update menu
                                if let Some(tray) = app.tray_by_id("main") {
                                    let menu = build_menu(app, &new_stats);
                                    let _ = tray.set_menu(Some(menu));

                                    let title = format!(
                                        "{}↑ {}↓",
                                        format_tokens(new_stats.total_usage.input_tokens),
                                        format_tokens(new_stats.total_usage.output_tokens)
                                    );
                                    let _ = tray.set_title(Some(&title));
                                }
                            }
                        }
                        "dashboard" => {
                            // Open web dashboard
                            let _ = open::that("http://localhost:3456");
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
                        // Refresh on click
                        if let Ok(new_stats) = get_stats() {
                            let app = tray.app_handle();
                            let menu = build_menu(app, &new_stats);
                            let _ = tray.set_menu(Some(menu));

                            let title = format!(
                                "{}↑ {}↓",
                                format_tokens(new_stats.total_usage.input_tokens),
                                format_tokens(new_stats.total_usage.output_tokens)
                            );
                            let _ = tray.set_title(Some(&title));
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

                            let title = format!(
                                "{}↑ {}↓",
                                format_tokens(new_stats.total_usage.input_tokens),
                                format_tokens(new_stats.total_usage.output_tokens)
                            );
                            let _ = tray.set_title(Some(&title));
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();
        }
    });
}
