// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod completion;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            completion::stream_chat,
            completion::balance,
            completion::title_genetation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
