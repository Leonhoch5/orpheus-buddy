mod commands;
mod state;
mod utils;

use utils::{start_callback_server, start_keyboard_listener};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

fn main() {
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // System tray
            let open_config = MenuItem::with_id(app, "open-config", "Open Config", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_config, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Orpheus Buddy")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open-config" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("notification-clicked", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("notification-clicked", ());
                    }
                })
                .build(app)?;

            start_callback_server();
            start_keyboard_listener(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dinosaurs::update_dinosaurs,
            commands::dinosaurs::clean_dinosaurs,
            commands::dinosaurs::get_resized_dinosaurs,
            commands::wakatime::get_wakatime_today,
            commands::wakatime::get_wakatime_today_detailed,
            commands::wakatime::get_wakatime_stats,
            commands::hackclub::start_hackclub_oauth,
            commands::hackclub::get_hackclub_auth_result,
            commands::hackclub::exchange_hackclub_oauth_code,
            commands::slack::start_slack_oauth,
            commands::slack::get_slack_auth_result,
            commands::slack::exchange_slack_oauth_code,
            commands::slack::start_slack_notification_poller,
            commands::party::check_party_time,
            commands::utils::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}