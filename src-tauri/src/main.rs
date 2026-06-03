mod commands;
mod state;
mod utils;

use utils::{start_callback_server, start_keyboard_listener};

fn main() {
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
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