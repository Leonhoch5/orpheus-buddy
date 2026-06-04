mod commands;
mod state;
mod utils;

use utils::{start_callback_server, start_keyboard_listener};
use tauri::{Emitter, Manager};

#[tauri::command]
async fn focus_window(app: tauri::AppHandle) {
    println!("[focus_window] called");
    if let Some(window) = app.get_webview_window("main") {
        println!("[focus_window] found window, focusing...");
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        println!("[focus_window] done");
    } else {
        println!("[focus_window] ERROR: could not find 'main' window");
    }
}

fn main() {
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            println!("[single-instance] callback fired!");
            println!("[single-instance] args: {:?}", args);
            println!("[single-instance] cwd: {:?}", cwd);

            if let Some(window) = app.get_webview_window("main") {
                println!("[single-instance] found window, showing + focusing");
                let show_result = window.show();
                let unmin_result = window.unminimize();
                let focus_result = window.set_focus();
                println!("[single-instance] show={:?} unminimize={:?} focus={:?}", show_result, unmin_result, focus_result);
            } else {
                println!("[single-instance] ERROR: could not find 'main' window");
            }

            let emit_result = app.emit("notification-clicked", ());
            println!("[single-instance] emit notification-clicked result: {:?}", emit_result);
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            println!("[setup] app starting up");
            let app_handle = app.handle().clone();
            let app_handle_clone = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                println!("[setup] emitting show-startup-notification");
                let _ = app_handle_clone.emit("show-startup-notification", ());
            });

            start_callback_server();
            start_keyboard_listener(app_handle);
            println!("[setup] done");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            focus_window,
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