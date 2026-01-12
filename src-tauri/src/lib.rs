use std::collections::HashMap;
use std::path::PathBuf;

use envhub_core::{InstallMode, State};
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_config() -> Result<State, String> {
    envhub_core::load_state().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(state: State) -> Result<(), String> {
    envhub_core::save_state(&state).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct AppInstallStatus {
    app_installed: HashMap<String, bool>,
}

fn bundled_launcher_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let file_name = if cfg!(windows) {
        "envhub-launcher.exe"
    } else {
        "envhub-launcher"
    };
    app.path()
        .resolve(file_name, BaseDirectory::Resource)
        .map_err(|e| e.to_string())
}

fn ensure_launcher_installed(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let platform = envhub_core::detect_platform(InstallMode::User)
        .map_err(|e| e.to_string())?;
    let launcher_name = if cfg!(windows) {
        "envhub-launcher.exe"
    } else {
        "envhub-launcher"
    };
    let installed_path = platform.install_dir.join(launcher_name);

    if installed_path.exists() {
        return Ok(installed_path);
    }

    let bundled_path = bundled_launcher_path(app)?;
    envhub_core::install_launcher(InstallMode::User, &bundled_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_install_status(app_names: Vec<String>) -> Result<AppInstallStatus, String> {
    let mut app_installed = HashMap::new();

    for name in app_names {
        let installed = envhub_core::is_shim_installed(&name, InstallMode::User);
        app_installed.insert(name, installed);
    }

    Ok(AppInstallStatus { app_installed })
}

#[tauri::command]
fn install_app_shim(app: tauri::AppHandle, app_name: String) -> Result<(), String> {
    let launcher_path = ensure_launcher_installed(&app)?;
    envhub_core::install_shim(&app_name, InstallMode::User, &launcher_path)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_config,
            save_config,
            get_app_install_status,
            install_app_shim
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
