// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod completion;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
#[tauri::command]
fn increment(counter: f32) -> f32 {
    counter + 2.0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, increment, completion::stream_chat, completion::balance])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
