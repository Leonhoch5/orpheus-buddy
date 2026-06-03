use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::Emitter;
use rdev::{listen, Event, EventType};

pub fn callback_server_script_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must have a parent directory")
        .join("callback")
        .join("https_server.py")
}

pub fn callback_auth_code_path() -> PathBuf {
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("orpheus-buddy");
        let _ = std::fs::create_dir_all(&dir);
        dir.push("hackclub_auth_code.txt");
        dir
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri must have a parent directory")
            .join("callback")
            .join("hackclub_auth_code.txt")
    }
}

pub fn slack_auth_code_path() -> PathBuf {
    if let Some(mut dir) = dirs::data_dir() {
        dir.push("orpheus-buddy");
        let _ = std::fs::create_dir_all(&dir);
        dir.push("slack_auth_code.txt");
        dir
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri must have a parent directory")
            .join("callback")
            .join("slack_auth_code.txt")
    }
}

pub fn start_callback_server() {
    let script_path = callback_server_script_path();

    if !script_path.exists() {
        eprintln!("Callback server script not found at {:?}", script_path);
        return;
    }

    for python_command in ["python", "py", "python3"] {
        match Command::new(python_command)
            .arg(&script_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(_) => {
                println!("Callback server started at http://localhost:3001/callback");
                return;
            }
            Err(_) => continue,
        }
    }

    eprintln!("Failed to start callback server with python, py, or python3");
}

pub fn start_keyboard_listener(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        listen(move |event: Event| {
            if let EventType::KeyPress(_) = event.event_type {
                app_handle.emit("global_keypress", {}).unwrap();
            }
        }).unwrap();
    });
}