use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use tauri::Emitter;

pub mod analyzer;
pub mod converter;
pub mod epub_path;
pub mod extractor;
pub mod image_handler;
pub mod models;
pub mod nav_builder;
pub mod opf_builder;
pub mod opf_document;
pub mod parser;
pub mod refactor;
pub mod resource_index;
pub mod resource_manager;
pub mod storage;
pub mod toc_manager;

pub use extractor::{convert_to_structure_result, extract_chapter_content};
pub use models::*;
pub use parser::parse_epub;
pub use refactor::refactor_epub;
pub use resource_manager::extract_refactored_chapter_content;
pub use storage::{cleanup_ogham_library, export_refactored_epub, get_epub_storage_path};
pub use toc_manager::TocManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpubInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub loaded_at: u64,
    pub epub_id: String,
    pub refactored_structure: RefactoredEpubResult,
}

/// 导入 EPUB 文件
///
/// 导入即完成标准化重构：验证原始 EPUB 后，立即拆解并整理到
/// Ogham 管理目录，后续前端操作都使用返回的 epub_id 和标准结构。
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
    let mut file = fs::File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;

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

    let loaded_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("[INFO] 开始导入并标准化 EPUB: {}", file_path);
    let refactored = refactor_epub(&file_path).map_err(|e| format!("EPUB 标准化失败: {}", e))?;
    let refactored_structure = refactored_result_from_refactored(&refactored);
    println!(
        "[INFO] EPUB 已导入管理目录: epub_id={}, storage={}",
        refactored.epub_id, refactored.storage_path
    );

    Ok(EpubInfo {
        id: refactored.epub_id.clone(),
        name: file_name,
        path: file_path,
        loaded_at,
        epub_id: refactored.epub_id.clone(),
        refactored_structure,
    })
}

fn chrono_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[tauri::command]
pub async fn import_epub_command(file_path: String) -> Result<EpubInfo, String> {
    tokio::task::spawn_blocking(move || import_epub(file_path))
        .await
        .map_err(|e| e.to_string())?
}

/// 解析 EPUB 结构
#[tauri::command]
pub async fn parse_epub_structure_command(
    epub_path: String,
) -> Result<EpubStructureResult, String> {
    let (parsed, opf_path) = parse_epub(&epub_path).map_err(|e| {
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

    extract_chapter_content(&epub_path, &chapter_path, &parsed.manifest).map_err(|e| e.to_string())
}

/// 重构 EPUB 文件
///
/// ⚠️ 重要说明：
/// 此命令会重新解析**原始 EPUB 文件**（epub_path），而不是读取缓存目录。
/// 适用于：首次加载 EPUB 或需要从原始文件重新生成标准结构的情况。
///
/// 工作流程：
/// 1. 解析原始 EPUB 文件（ZIP 格式）
/// 2. 分析 EPUB 结构
/// 3. 创建标准目录结构到缓存目录
/// 4. 生成新的 OPF、nav.xhtml、toc.ncx 文件
///
/// 注意：执行此操作后，会生成新的 epub_id（基于时间戳）
#[tauri::command]
pub async fn refactor_epub_command(epub_path: String) -> Result<RefactoredEpubResult, String> {
    let refactored = tokio::task::spawn_blocking(move || refactor_epub(&epub_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(refactored_result_from_refactored(&refactored))
}

/// 从重构后的 EPUB 获取章节内容
#[tauri::command]
pub async fn get_chapter_from_refactored_command(
    epub_id: String,
    chapter_path: String,
) -> Result<ChapterContent, String> {
    let storage_path = get_epub_storage_path(&epub_id);

    // 提取章节内容和资源（含 Base64 转换）
    extract_refactored_chapter_content(&storage_path, &chapter_path).map_err(|e| e.to_string())
}

/// 导出重构后的 EPUB 文件
#[tauri::command]
pub async fn export_epub_command(epub_id: String, export_path: String) -> Result<String, String> {
    export_refactored_epub(&epub_id, &export_path).map_err(|e| {
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

    let file = File::open(&epub_path).map_err(|e| format!("无法打开 EPUB 文件: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("无法打开 ZIP 归档: {}", e))?;

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
    let buffer =
        fs::read(&full_path).map_err(|e| format!("无法读取图片 {:?}: {}", full_path, e))?;

    // 转换为 Base64
    let base64_engine = base64::engine::general_purpose::STANDARD;
    let base64_data = base64_engine.encode(&buffer);

    Ok(base64_data)
}

/// 从当前章节解析 EPUB 内部链接，返回标准化后的章节路径和锚点
#[tauri::command]
pub async fn resolve_chapter_href_command(
    epub_id: String,
    current_chapter_path: Option<String>,
    href: String,
) -> Result<ResolvedChapterHrefDto, String> {
    let storage_path = get_epub_storage_path(&epub_id);
    let document =
        opf_document::load_opf_document(&storage_path, &epub_id).map_err(|e| e.to_string())?;

    resolve_chapter_href(&document.manifest, current_chapter_path.as_deref(), &href)
        .map_err(|e| e.to_string())
}

/// 加载目录结构（带文件映射）
#[tauri::command]
pub async fn load_toc_entries_command(epub_id: String) -> Result<Vec<TocChapterDto>, String> {
    let storage_path = get_epub_storage_path(&epub_id);

    TocManager::load_toc_from_storage(&storage_path).map_err(|e| e.to_string())
}

fn resolve_chapter_href(
    manifest: &StandardManifest,
    current_chapter_path: Option<&str>,
    href: &str,
) -> Result<ResolvedChapterHrefDto, RefactorError> {
    let trimmed_href = href.trim();
    if trimmed_href.is_empty() {
        return Ok(ResolvedChapterHrefDto {
            chapter_path: None,
            anchor: None,
            same_chapter: false,
        });
    }

    let (path_part, _) = epub_path::split_reference_suffix(trimmed_href);
    let anchor = epub_path::reference_anchor(trimmed_href);

    if path_part.is_empty() {
        return Ok(ResolvedChapterHrefDto {
            chapter_path: current_chapter_path.map(ToString::to_string),
            anchor,
            same_chapter: true,
        });
    }

    let resolved_path = resolve_chapter_reference_path(current_chapter_path, path_part);
    let chapter_path =
        resolve_manifest_chapter_path(manifest, &resolved_path).ok_or_else(|| {
            RefactorError::MissingFile(format!("链接目标章节不存在: {}", trimmed_href))
        })?;

    let same_chapter = current_chapter_path
        .map(|current| epub_path::casefold(current) == epub_path::casefold(&chapter_path))
        .unwrap_or(false);

    Ok(ResolvedChapterHrefDto {
        chapter_path: Some(chapter_path),
        anchor,
        same_chapter,
    })
}

fn resolve_chapter_reference_path(current_chapter_path: Option<&str>, path_part: &str) -> String {
    let normalized_path = epub_path::normalize(path_part);

    if normalized_path.starts_with("OEBPS/") || epub_path::has_standard_dir_prefix(&normalized_path)
    {
        return epub_path::manifest_path(&normalized_path);
    }

    let base_dir = current_chapter_path
        .map(epub_path::parent)
        .unwrap_or_else(|| "OEBPS/Text".to_string());
    epub_path::resolve_relative(&base_dir, &normalized_path)
}

fn resolve_manifest_chapter_path(manifest: &StandardManifest, target_path: &str) -> Option<String> {
    let target = epub_path::casefold(target_path);
    manifest
        .items
        .iter()
        .filter(|item| is_renderable_chapter_item(item))
        .map(|item| format!("OEBPS/{}", item.href))
        .find(|standard_path| epub_path::casefold(standard_path) == target)
}

/// 更新目录顺序并同步到 OPF 和导航文件
#[tauri::command]
pub async fn update_toc_order_command(
    epub_id: String,
    new_order: Vec<TocOrderDto>,
) -> Result<(), String> {
    toc_manager::update_toc_order(&epub_id, &new_order).map_err(|e| e.to_string())
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

/// 简繁转换命令
#[tauri::command]
pub async fn convert_simplified_traditional_command(
    app: tauri::AppHandle,
    epub_id: String,
    mode: String, // "s2t" 或 "t2s"
    task_id: String,
) -> Result<converter::ConversionResult, String> {
    let conversion_mode = converter::ConversionMode::from_str(&mode)?;
    let task_id = if task_id.trim().is_empty() {
        format!("conversion-{}", chrono_timestamp_millis())
    } else {
        task_id
    };
    let app_handle = app.clone();
    let task_id_for_worker = task_id.clone();

    tokio::task::spawn_blocking(move || {
        converter::convert_epub_with_progress(
            &epub_id,
            conversion_mode,
            &task_id_for_worker,
            |progress| {
                if let Err(err) = app_handle.emit("conversion_progress", &progress) {
                    eprintln!("[WARN] 发送简繁转换进度事件失败: {}", err);
                }
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 处理整本 EPUB 中所有章节的图片链接
#[tauri::command]
pub async fn process_all_images_command(
    app: tauri::AppHandle,
    epub_id: String,
    task_id: String,
) -> Result<ImageProcessResult, String> {
    let task_id = if task_id.trim().is_empty() {
        format!("image-process-{}", chrono_timestamp_millis())
    } else {
        task_id
    };
    let app_handle = app.clone();
    let epub_id = epub_id.clone();
    let task_id_for_worker = task_id.clone();

    tokio::task::spawn_blocking(move || {
        image_handler::process_all_images(&epub_id, &task_id_for_worker, |progress| {
            if let Err(err) = app_handle.emit("image_process_progress", &progress) {
                eprintln!("[WARN] 发送图片处理进度事件失败: {}", err);
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// 从缓存目录重新加载 EPUB 结构（不重新解析原始文件）
///
/// ⚠️ 重要说明：
/// 此命令从**缓存目录**（/tmp/ogham-library/{epub_id}/）读取当前的文件结构，
/// 而不是重新解析原始 EPUB 文件。
/// 适用于：简繁转换、图片处理等操作后，需要刷新前端视图的情况。
///
/// 与 refactor_epub_command 的区别：
/// - refactor_epub_command：解析原始 EPUB 文件，生成新的缓存和 epub_id
/// - reload_epub_structure_command：读取现有缓存，使用相同的 epub_id
///
/// 工作流程：
/// 1. 从缓存目录 OEBPS/content.opf 读取 manifest 和 spine
/// 2. 从缓存目录 OEBPS/nav.xhtml 或 toc.ncx 读取目录结构
/// 3. 返回当前缓存中的最新文件结构
///
/// 注意：此操作优先复用 .ogham/resource_index.json；索引不存在或 epub_id 不匹配时自动重建。
/// 会改写资源关系的命令应在落盘后主动刷新资源索引。
#[tauri::command]
pub async fn reload_epub_structure_command(
    epub_id: String,
) -> Result<RefactoredEpubResult, String> {
    let storage_path = get_epub_storage_path(&epub_id);
    load_refactored_result_from_cache(&epub_id, &storage_path)
}

fn load_refactored_result_from_cache(
    epub_id: &str,
    storage_path: &str,
) -> Result<RefactoredEpubResult, String> {
    let oebps_path = Path::new(storage_path).join("OEBPS");

    if !oebps_path.exists() {
        return Err(format!("EPUB 缓存目录不存在: {}", oebps_path.display()));
    }

    let _workspace_metadata =
        storage::load_workspace_metadata(storage_path).map_err(|e| e.to_string())?;

    let opf_document =
        opf_document::load_opf_document(storage_path, epub_id).map_err(|e| e.to_string())?;
    let metadata = opf_document.metadata;
    let manifest = opf_document.manifest;
    let spine = opf_document.spine;

    // 加载导航。所有缓存读取入口都通过 TocManager，避免 nav.xhtml、toc.ncx 和前端 TOC
    // 各自解析出不同结构。
    let navigation = load_navigation_from_cache(storage_path)?;

    let structure = build_standard_epub_structure(&manifest, &spine, navigation);

    resource_index::load_or_refresh_resource_index(epub_id, storage_path)
        .map_err(|e| e.to_string())?;

    Ok(RefactoredEpubResult {
        epub_id: epub_id.to_string(),
        metadata,
        structure,
        storage_path: storage_path.to_string(),
    })
}

fn refactored_result_from_refactored(refactored: &RefactoredEpub) -> RefactoredEpubResult {
    let navigation = toc_entries_to_navigation_entries(&refactored.navigation.toc);
    let structure =
        build_standard_epub_structure(&refactored.manifest, &refactored.spine, navigation);

    RefactoredEpubResult {
        epub_id: refactored.epub_id.clone(),
        metadata: refactored.metadata.clone(),
        structure,
        storage_path: refactored.storage_path.clone(),
    }
}

fn build_standard_epub_structure(
    manifest: &StandardManifest,
    spine: &StandardSpine,
    navigation: Vec<NavigationEntry>,
) -> StandardEpubStructure {
    let title_by_href = collect_navigation_titles_by_href(&navigation);
    let spine_order_by_id: HashMap<&str, usize> = spine
        .itemrefs
        .iter()
        .enumerate()
        .map(|(index, spine_item)| (spine_item.idref.as_str(), index))
        .collect();

    let chapters: Vec<StandardChapter> = manifest
        .items
        .iter()
        .filter(|item| is_renderable_chapter_item(item))
        .filter_map(|item| {
            let title = title_by_href.get(&epub_path::casefold(&item.href)).cloned();

            let original_filename = item
                .href
                .rsplit('/')
                .next()
                .unwrap_or(&item.href)
                .to_string();
            let standard_path = format!("OEBPS/{}", item.href);

            let order = spine_order_by_id
                .get(item.id.as_str())
                .copied()
                .unwrap_or(spine.itemrefs.len());

            Some(StandardChapter {
                id: item.id.clone(),
                original_filename,
                standard_path,
                title,
                order,
            })
        })
        .collect();

    let styles: Vec<String> = manifest
        .items
        .iter()
        .filter(|item| item.media_type == "text/css")
        .map(|item| format!("OEBPS/{}", item.href))
        .collect();

    let images: Vec<String> = manifest
        .items
        .iter()
        .filter(|item| item.media_type.starts_with("image/"))
        .map(|item| format!("OEBPS/{}", item.href))
        .collect();

    StandardEpubStructure {
        chapters,
        styles,
        images,
        fonts: manifest
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.media_type.as_str(),
                    "font/ttf"
                        | "font/otf"
                        | "font/woff"
                        | "font/woff2"
                        | "application/font-ttf"
                        | "application/font-woff"
                        | "application/font-woff2"
                )
            })
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        navigation,
    }
}

/// 读取当前管理目录中的全局资源关系索引；不存在或 epub_id 不匹配时自动重建
#[tauri::command]
pub async fn load_resource_index_command(epub_id: String) -> Result<ResourceIndex, String> {
    let storage_path = get_epub_storage_path(&epub_id);
    resource_index::load_or_refresh_resource_index(&epub_id, &storage_path)
        .map_err(|e| e.to_string())
}

/// 从缓存目录加载导航结构
fn load_navigation_from_cache(storage_path: &str) -> Result<Vec<NavigationEntry>, String> {
    match TocManager::load_toc_from_storage(storage_path) {
        Ok(entries) => Ok(toc_chapters_to_navigation(&entries)),
        Err(RefactorError::MissingFile(_)) => Ok(Vec::new()),
        Err(error) => Err(error.to_string()),
    }
}

fn toc_chapters_to_navigation(entries: &[TocChapterDto]) -> Vec<NavigationEntry> {
    entries
        .iter()
        .map(|entry| NavigationEntry {
            id: entry.id.clone(),
            label: entry.label.clone(),
            content_src: entry.content_src.clone(),
            level: entry.level,
            children: toc_chapters_to_navigation(&entry.children),
        })
        .collect()
}

fn toc_entries_to_navigation_entries(entries: &[TocEntry]) -> Vec<NavigationEntry> {
    entries
        .iter()
        .map(|entry| NavigationEntry {
            id: entry.id.clone(),
            label: entry.label.clone(),
            content_src: entry.content_src.clone(),
            level: entry.level,
            children: toc_entries_to_navigation_entries(&entry.children),
        })
        .collect()
}

fn is_renderable_chapter_item(item: &StandardManifestItem) -> bool {
    (item.media_type == "application/xhtml+xml" || item.media_type == "text/html")
        && item.href.replace('\\', "/").starts_with("Text/")
        && !item
            .properties
            .as_ref()
            .map(|props| props.iter().any(|prop| prop == "nav"))
            .unwrap_or(false)
}

fn collect_navigation_titles_by_href(entries: &[NavigationEntry]) -> HashMap<String, String> {
    let mut titles = HashMap::new();
    collect_navigation_titles_by_href_recursive(entries, &mut titles);
    titles
}

fn collect_navigation_titles_by_href_recursive(
    entries: &[NavigationEntry],
    titles: &mut HashMap<String, String>,
) {
    for entry in entries {
        let href = epub_path::casefold(&entry.content_src);
        if !href.is_empty() {
            titles.entry(href).or_insert_with(|| entry.label.clone());
        }

        collect_navigation_titles_by_href_recursive(&entry.children, titles);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> StandardManifest {
        StandardManifest {
            items: vec![
                chapter_item("chapter-1", "Text/ch1.xhtml"),
                chapter_item("chapter-2", "Text/ch2.xhtml"),
                StandardManifestItem {
                    id: "nav".to_string(),
                    href: "nav.xhtml".to_string(),
                    media_type: "application/xhtml+xml".to_string(),
                    properties: Some(vec!["nav".to_string()]),
                },
            ],
        }
    }

    fn chapter_item(id: &str, href: &str) -> StandardManifestItem {
        StandardManifestItem {
            id: id.to_string(),
            href: href.to_string(),
            media_type: "application/xhtml+xml".to_string(),
            properties: None,
        }
    }

    #[test]
    fn resolves_current_chapter_relative_href() {
        let resolved = resolve_chapter_href(
            &test_manifest(),
            Some("OEBPS/Text/ch1.xhtml"),
            "ch2.xhtml#p2",
        )
        .unwrap();

        assert_eq!(
            resolved.chapter_path.as_deref(),
            Some("OEBPS/Text/ch2.xhtml")
        );
        assert_eq!(resolved.anchor.as_deref(), Some("p2"));
        assert!(!resolved.same_chapter);
    }

    #[test]
    fn resolves_opf_relative_text_href_without_double_text_prefix() {
        let resolved = resolve_chapter_href(
            &test_manifest(),
            Some("OEBPS/Text/ch1.xhtml"),
            "Text/ch2.xhtml?x=1#p2",
        )
        .unwrap();

        assert_eq!(
            resolved.chapter_path.as_deref(),
            Some("OEBPS/Text/ch2.xhtml")
        );
        assert_eq!(resolved.anchor.as_deref(), Some("p2"));
        assert!(!resolved.same_chapter);
    }

    #[test]
    fn resolves_fragment_only_href_to_current_chapter() {
        let resolved =
            resolve_chapter_href(&test_manifest(), Some("OEBPS/Text/ch1.xhtml"), "#p1").unwrap();

        assert_eq!(
            resolved.chapter_path.as_deref(),
            Some("OEBPS/Text/ch1.xhtml")
        );
        assert_eq!(resolved.anchor.as_deref(), Some("p1"));
        assert!(resolved.same_chapter);
    }
}
