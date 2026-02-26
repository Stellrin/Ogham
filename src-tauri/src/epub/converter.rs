use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use zhconv::{zhconv, Variant};

use super::storage::get_epub_storage_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionResult {
    pub success: bool,
    pub files_converted: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ConversionMode {
    SimplifiedToTraditional,
    TraditionalToSimplified,
}

impl ConversionMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "s2t" => Ok(ConversionMode::SimplifiedToTraditional),
            "t2s" => Ok(ConversionMode::TraditionalToSimplified),
            _ => Err(format!("无效的转换模式: {}, 支持的模式: s2t, t2s", s)),
        }
    }

    fn to_zhconv_target(&self) -> Variant {
        match self {
            ConversionMode::SimplifiedToTraditional => Variant::ZhHant,
            ConversionMode::TraditionalToSimplified => Variant::ZhCN,
        }
    }
}

/// 转换单个文件的内容
pub fn convert_file_content(content: &str, mode: ConversionMode) -> String {
    let target = mode.to_zhconv_target();
    zhconv(content, target)
}

/// 转换 EPUB 中的文本文件
pub fn convert_epub(epub_id: &str, mode: ConversionMode) -> Result<ConversionResult, String> {
    let storage_path = get_epub_storage_path(epub_id);

    if !Path::new(&storage_path).exists() {
        return Err(format!("EPUB 存储路径不存在: {}", storage_path));
    }

    let mut files_converted = 0usize;

    // 需要转换的文件扩展名
    let text_extensions = ["xhtml", "html", "xml", "css", "ncx", "opf"];

    // 遍历 OEBPS 目录下的所有文件
    let oebps_path = Path::new(&storage_path).join("OEBPS");
    if oebps_path.exists() {
        convert_directory_recursive(&oebps_path, mode, &text_extensions, &mut files_converted)?;
    }

    Ok(ConversionResult {
        success: true,
        files_converted,
        error: None,
    })
}

/// 递归转换目录中的文本文件
fn convert_directory_recursive(
    dir_path: &Path,
    mode: ConversionMode,
    text_extensions: &[&str],
    files_converted: &mut usize,
) -> Result<(), String> {
    let entries = fs::read_dir(dir_path).map_err(|e| format!("无法读取目录 {:?}: {}", dir_path, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("无法读取目录项: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // 递归处理子目录
            convert_directory_recursive(&path, mode, text_extensions, files_converted)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if text_extensions.contains(&ext_str.as_str()) {
                // 读取文件内容
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("无法读取文件 {:?}: {}", path, e))?;

                // 转换内容
                let converted = convert_file_content(&content, mode);

                // 写回文件
                fs::write(&path, converted)
                    .map_err(|e| format!("无法写入文件 {:?}: {}", path, e))?;

                *files_converted += 1;
            }
        }
    }

    Ok(())
}
