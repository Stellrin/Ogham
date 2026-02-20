use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use std::io::Read;

pub mod models;
pub mod parser;
pub mod extractor;
pub mod analyzer;
pub mod refactor;
pub mod opf_builder;
pub mod nav_builder;
pub mod resource_manager;
pub mod storage;
pub mod toc_manager;

pub use models::*;
pub use parser::parse_epub;
pub use extractor::{extract_chapter_content, convert_to_structure_result};
pub use refactor::{refactor_epub, convert_to_refactored_result};
pub use storage::{get_epub_storage_path, export_refactored_epub, cleanup_ogham_library};
pub use resource_manager::{read_refactored_file as read_resource_file_content, extract_refactored_chapter_content};
pub use toc_manager::TocManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpubInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub loaded_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub epub_info: Option<EpubInfo>,
    pub error: Option<String>,
}

/// 导入 EPUB 文件
/// 目前只验证文件是否存在，不做解析
pub fn import_epub(file_path: String) -> Result<EpubInfo, String> {
    let path = Path::new(&file_path);

    // 检查文件是否存在
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    // 检查是否是文件（不是目录）
    if !path.is_file() {
        return Err(format!("路径不是文件: {}", file_path));
    }

    // 检查文件扩展名
    match path.extension() {
        Some(ext) if ext.eq_ignore_ascii_case("epub") => {}
        Some(ext) => {
            return Err(format!("文件扩展名不正确: {:?}, 期望: epub", ext));
        }
        None => {
            return Err(format!("文件没有扩展名: {}", file_path));
        }
    }

    // 读取文件前几个字节来验证 EPUB 格式
    let mut file = fs::File::open(path)
        .map_err(|e| format!("无法打开文件: {}", e))?;

    let mut buffer = [0; 4];
    file.read_exact(&mut buffer)
        .map_err(|e| format!("无法读取文件: {}", e))?;

    // EPUB 文件应该以 PK\x03\x04 开头（ZIP 格式）
    if &buffer != b"PK\x03\x04" {
        return Err(format!("无效的 EPUB 文件格式: {}", file_path));
    }

    // 获取文件名
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // 生成唯一 ID
    let id = format!("{}-{}", file_name, chrono_timestamp());

    Ok(EpubInfo {
        id,
        name: file_name,
        path: file_path,
        loaded_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// 生成简单的时间戳（避免额外依赖）
fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tauri::command]
pub async fn import_epub_command(file_path: String) -> ImportResult {
    match import_epub(file_path) {
        Ok(epub_info) => ImportResult {
            success: true,
            epub_info: Some(epub_info),
            error: None,
        },
        Err(error) => ImportResult {
            success: false,
            epub_info: None,
            error: Some(error),
        },
    }
}

/// 解析 EPUB 结构
#[tauri::command]
pub async fn parse_epub_structure_command(epub_path: String) -> Result<EpubStructureResult, String> {

    let (parsed, opf_path) = parse_epub(&epub_path)
        .map_err(|e| {
            println!("[ERROR] EPUB 解析失败: {}", e);
            e.to_string()
        })?;

    Ok(convert_to_structure_result(&parsed, &opf_path))
}

/// 获取章节内容（含嵌入的 Base64 资源）
#[tauri::command]
pub async fn get_chapter_content_command(
    epub_path: String,
    chapter_path: String,
) -> Result<ChapterContent, String> {
    // 首先解析 EPUB 获取 manifest（忽略 opf_path）
    let (parsed, _) = parse_epub(&epub_path).map_err(|e| e.to_string())?;

    extract_chapter_content(&epub_path, &chapter_path, &parsed.manifest)
        .map_err(|e| e.to_string())
}

/// 重构 EPUB 文件
#[tauri::command]
pub async fn refactor_epub_command(
    epub_path: String,
) -> Result<RefactoredEpubResult, String> {

    let refactored = refactor_epub(&epub_path)
        .map_err(|e| {
            e.to_string()
        })?;


    Ok(convert_to_refactored_result(&refactored))
}

/// 从重构后的 EPUB 获取章节内容
#[tauri::command]
pub async fn get_chapter_from_refactored_command(
    epub_id: String,
    chapter_path: String,
) -> Result<ChapterContent, String> {

    let storage_path = get_epub_storage_path(&epub_id);

    // 提取章节内容和资源（含 Base64 转换）
    extract_refactored_chapter_content(&storage_path, &chapter_path)
        .map_err(|e| e.to_string())
}

/// 导出重构后的 EPUB 文件
#[tauri::command]
pub async fn export_epub_command(
    epub_id: String,
    export_path: String,
) -> Result<String, String> {

    export_refactored_epub(&epub_id, &export_path)
        .map_err(|e| {
            println!("[ERROR] EPUB 导出失败: {}", e);
            e.to_string()
        })?;


    Ok(export_path)
}

/// 从原始 EPUB 获取图片内容（返回 Base64）
#[tauri::command]
pub async fn get_image_content_command(
    epub_path: String,
    image_path: String,
) -> Result<String, String> {
    use base64::Engine;
    use std::io::Read;
    use zip::ZipArchive;

    let file = File::open(&epub_path)
        .map_err(|e| format!("无法打开 EPUB 文件: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("无法打开 ZIP 归档: {}", e))?;

    // 从 ZIP 中提取图片
    let mut image_file = archive
        .by_name(&image_path)
        .map_err(|_| format!("图片文件不存在: {}", image_path))?;

    let mut buffer = Vec::new();
    image_file
        .read_to_end(&mut buffer)
        .map_err(|e| format!("无法读取图片: {}", e))?;

    // 转换为 Base64
    let base64_engine = base64::engine::general_purpose::STANDARD;
    let base64_data = base64_engine.encode(&buffer);

    Ok(base64_data)
}

/// 从重构后的 EPUB 获取图片内容（返回 Base64）
#[tauri::command]
pub async fn get_image_from_refactored_command(
    epub_id: String,
    image_path: String,
) -> Result<String, String> {
    use base64::Engine;

    let storage_path = get_epub_storage_path(&epub_id);
    let full_path = Path::new(&storage_path).join(&image_path);

    // 读取图片文件
    let buffer = fs::read(&full_path)
        .map_err(|e| format!("无法读取图片 {:?}: {}", full_path, e))?;

    // 转换为 Base64
    let base64_engine = base64::engine::general_purpose::STANDARD;
    let base64_data = base64_engine.encode(&buffer);

    Ok(base64_data)
}

/// 加载目录结构（带文件映射）
#[tauri::command]
pub async fn load_toc_entries_command(
    epub_id: String,
) -> Result<Vec<TocChapterDto>, String> {
    let storage_path = get_epub_storage_path(&epub_id);

    TocManager::load_toc_with_file_mapping(&epub_id, &storage_path)
        .map_err(|e| e.to_string())
}

/// 更新目录顺序并同步到 OPF 和导航文件
#[tauri::command]
pub async fn update_toc_order_command(
    epub_id: String,
    new_order: Vec<TocOrderDto>,
) -> Result<(), String> {
    toc_manager::update_toc_order(&epub_id, &new_order)
        .map_err(|e| e.to_string())
}

/// 更新单个目录项（标签、文件对应关系）
#[tauri::command]
pub async fn update_toc_entry_command(
    epub_id: String,
    entry_id: String,
    new_label: Option<String>,
    new_content_src: Option<String>,
) -> Result<(), String> {
    toc_manager::update_toc_entry(
        &epub_id,
        &entry_id,
        new_label.as_deref(),
        new_content_src.as_deref(),
    )
    .map_err(|e| e.to_string())
}
