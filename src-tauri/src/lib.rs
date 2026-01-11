use envhub_core::State;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_config, save_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
