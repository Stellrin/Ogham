mod epub;

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tauri::{Emitter, Manager};

const EPUB_OPEN_REQUESTED_EVENT: &str = "epub-open-requested";

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let cwd = PathBuf::from(cwd);
            let paths = collect_epub_open_paths(args, Some(cwd.as_path()));

            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.show();
                let _ = main_window.unminimize();
                let _ = main_window.set_focus();
            }

            if !paths.is_empty() {
                println!("[INFO] 收到系统打开 EPUB 请求: {:?}", paths);
                if let Err(err) = app.emit(EPUB_OPEN_REQUESTED_EVENT, paths) {
                    eprintln!("[WARN] 发送 EPUB 打开事件失败: {}", err);
                }
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 监听窗口关闭事件，清理临时目录
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Destroyed = event {
                        if let Err(e) = epub::cleanup_ogham_library() {
                            eprintln!("[ERROR] 清理临时目录失败: {}", e);
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            epub::import_epub_command,
            epub::parse_epub_structure_command,
            epub::get_chapter_content_command,
            epub::refactor_epub_command,
            epub::reload_epub_structure_command,
            epub::get_chapter_from_refactored_command,
            epub::export_epub_command,
            epub::get_image_content_command,
            epub::get_image_from_refactored_command,
            epub::resolve_chapter_href_command,
            epub::load_toc_entries_command,
            epub::update_toc_order_command,
            epub::update_toc_entry_command,
            epub::convert_simplified_traditional_command,
            epub::process_all_images_command,
            epub::load_resource_index_command,
            get_startup_epub_paths_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_startup_epub_paths_command() -> Vec<String> {
    let cwd = std::env::current_dir().ok();
    let paths = collect_epub_open_paths(std::env::args_os().skip(1), cwd.as_deref());

    if !paths.is_empty() {
        println!("[INFO] 启动参数中发现 EPUB: {:?}", paths);
    }

    paths
}

fn collect_epub_open_paths<I, S>(args: I, cwd: Option<&Path>) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut seen = HashSet::new();
    let mut paths = Vec::new();

    for arg in args {
        if let Some(path) = open_arg_to_epub_path(arg.into(), cwd) {
            let key = path.to_lowercase();
            if seen.insert(key) {
                paths.push(path);
            }
        }
    }

    paths
}

fn open_arg_to_epub_path(arg: OsString, cwd: Option<&Path>) -> Option<String> {
    let raw = arg.as_os_str().to_string_lossy();
    let trimmed = raw.trim().trim_matches('"');

    if trimmed.is_empty() {
        return None;
    }

    let mut path = open_arg_to_path(trimmed);
    if path.is_relative() {
        if let Some(cwd) = cwd {
            path = cwd.join(path);
        }
    }

    let is_epub = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("epub"))
        .unwrap_or(false);

    if is_epub && path.is_file() {
        Some(path.to_string_lossy().into_owned())
    } else {
        None
    }
}

fn open_arg_to_path(arg: &str) -> PathBuf {
    if let Some(file_path) = arg.strip_prefix("file://") {
        #[cfg(windows)]
        {
            return PathBuf::from(file_path.trim_start_matches('/'));
        }

        #[cfg(not(windows))]
        {
            return PathBuf::from(file_path);
        }
    }

    PathBuf::from(arg)
}
