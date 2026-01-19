mod epub;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 监听窗口关闭事件，清理临时目录
            let main_window = app.get_webview_window("main").unwrap();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    if let Err(e) = epub::cleanup_ogham_library() {
                        eprintln!("[ERROR] 清理临时目录失败: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            epub::import_epub_command,
            epub::parse_epub_structure_command,
            epub::get_chapter_content_command,
            epub::test_command,
            epub::refactor_epub_command,
            epub::get_chapter_from_refactored_command,
            epub::export_epub_command,
            epub::get_image_content_command,
            epub::get_image_from_refactored_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
