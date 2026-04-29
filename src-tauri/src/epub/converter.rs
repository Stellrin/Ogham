use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zhconv::{zhconv, Variant};

use super::storage::get_epub_storage_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionResult {
    pub task_id: String,
    pub files_converted: usize,
    pub total_files: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionProgress {
    pub task_id: String,
    pub file_path: Option<String>,
    pub processed_files: usize,
    pub total_files: usize,
    pub files_converted: usize,
    pub progress: u32,
    pub stage: String,
    pub message: String,
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

pub fn convert_epub_with_progress<F>(
    epub_id: &str,
    mode: ConversionMode,
    task_id: &str,
    mut progress_callback: F,
) -> Result<ConversionResult, String>
where
    F: FnMut(ConversionProgress),
{
    let storage_path = get_epub_storage_path(epub_id);

    if !Path::new(&storage_path).exists() {
        return Err(format!("EPUB 存储路径不存在: {}", storage_path));
    }

    let mut files_converted = 0usize;

    // 需要转换的文件扩展名
    let text_extensions = ["xhtml", "html", "xml", "css", "ncx", "opf"];
    let storage_root = Path::new(&storage_path);

    // 遍历 OEBPS 目录下的所有文件
    let oebps_path = storage_root.join("OEBPS");
    let mut text_files = Vec::new();
    if oebps_path.exists() {
        collect_text_files_recursive(&oebps_path, &text_extensions, &mut text_files)?;
    }
    text_files.sort();

    let total_files = text_files.len();
    emit_progress(
        &mut progress_callback,
        task_id,
        None,
        0,
        total_files,
        0,
        "started",
        "开始简繁转换",
    );

    for (index, path) in text_files.iter().enumerate() {
        let content =
            fs::read_to_string(path).map_err(|e| format!("无法读取文件 {:?}: {}", path, e))?;
        let converted = convert_file_content(&content, mode);
        fs::write(path, converted).map_err(|e| format!("无法写入文件 {:?}: {}", path, e))?;

        files_converted += 1;
        let processed_files = index + 1;
        emit_progress(
            &mut progress_callback,
            task_id,
            Some(display_relative_path(storage_root, path)),
            processed_files,
            total_files,
            files_converted,
            "converting",
            "正在转换文本文件",
        );
    }

    emit_progress(
        &mut progress_callback,
        task_id,
        None,
        total_files,
        total_files,
        files_converted,
        "completed",
        "简繁转换完成",
    );

    Ok(ConversionResult {
        task_id: task_id.to_string(),
        files_converted,
        total_files,
    })
}

fn emit_progress<F>(
    progress_callback: &mut F,
    task_id: &str,
    file_path: Option<String>,
    processed_files: usize,
    total_files: usize,
    files_converted: usize,
    stage: &str,
    message: &str,
) where
    F: FnMut(ConversionProgress),
{
    let progress = if total_files > 0 {
        ((processed_files as f64 / total_files as f64) * 100.0).round() as u32
    } else if stage == "completed" {
        100
    } else {
        0
    };

    progress_callback(ConversionProgress {
        task_id: task_id.to_string(),
        file_path,
        processed_files,
        total_files,
        files_converted,
        progress: progress.min(100),
        stage: stage.to_string(),
        message: message.to_string(),
    });
}

fn display_relative_path(storage_root: &Path, path: &Path) -> String {
    path.strip_prefix(storage_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// 递归收集目录中的文本文件
fn collect_text_files_recursive(
    dir_path: &Path,
    text_extensions: &[&str],
    text_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(dir_path).map_err(|e| format!("无法读取目录 {:?}: {}", dir_path, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("无法读取目录项: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // 递归处理子目录
            collect_text_files_recursive(&path, text_extensions, text_files)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if text_extensions.contains(&ext_str.as_str()) {
                text_files.push(path);
            }
        }
    }

    Ok(())
}
