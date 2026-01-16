use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Read;

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
