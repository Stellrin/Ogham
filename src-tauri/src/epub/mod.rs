use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use std::io::Read;
use tauri::Emitter;

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
pub mod converter;
pub mod image_handler;

pub use models::*;
pub use parser::parse_epub;
pub use extractor::{extract_chapter_content, convert_to_structure_result};
pub use refactor::{refactor_epub, convert_to_refactored_result};
pub use storage::{get_epub_storage_path, export_refactored_epub, cleanup_ogham_library};
pub use resource_manager::{read_refactored_file as read_resource_file_content, extract_refactored_chapter_content};
pub use toc_manager::TocManager;
pub use converter::convert_epub;
pub use image_handler::process_all_images;

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

fn chrono_timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
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

/// 简繁转换命令
#[tauri::command]
pub async fn convert_simplified_traditional_command(
    epub_id: String,
    mode: String, // "s2t" 或 "t2s"
) -> Result<converter::ConversionResult, String> {
    let conversion_mode = converter::ConversionMode::from_str(&mode)?;

    converter::convert_epub(&epub_id, conversion_mode)
}

/// 处理整本 EPUB 中所有章节的图片链接
#[tauri::command]
pub async fn process_all_images_command(
    app: tauri::AppHandle,
    epub_id: String,
) -> Result<ImageProcessResult, String> {
    let task_id = format!("image-process-{}", chrono_timestamp_millis());
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
/// 注意：此操作不会修改任何文件，仅读取现有缓存
#[tauri::command]
pub async fn reload_epub_structure_command(
    epub_id: String,
) -> Result<RefactoredEpubResult, String> {
    let storage_path = get_epub_storage_path(&epub_id);
    let oebps_path = Path::new(&storage_path).join("OEBPS");

    if !oebps_path.exists() {
        return Err(format!("EPUB 缓存目录不存在: {}", oebps_path.display()));
    }

    // 加载 OPF 文件获取 manifest 和 spine
    let opf_path = oebps_path.join("content.opf");
    let opf_content = fs::read_to_string(&opf_path)
        .map_err(|e| format!("无法读取 content.opf: {}", e))?;

    // 解析 manifest
    let mut manifest_items = Vec::new();
    let mut search_pos = 0;
    while let Some(item_start) = opf_content[search_pos..].find("<item ") {
        let item_start = search_pos + item_start;
        let item_end = opf_content[item_start..].find("/>")
            .map(|end| item_start + end + 2)
            .unwrap_or(opf_content.len());

        let item_content = &opf_content[item_start..item_end];

        let id = extract_xml_attr(item_content, "id").unwrap_or_default();
        let href = extract_xml_attr(item_content, "href").unwrap_or_default();
        let media_type = extract_xml_attr(item_content, "media-type").unwrap_or_default();
        let properties_str = extract_xml_attr(item_content, "properties").unwrap_or_default();

        if !id.is_empty() && !href.is_empty() {
            let properties: Option<Vec<String>> = if properties_str.is_empty() {
                None
            } else {
                Some(properties_str.split_whitespace().map(String::from).collect())
            };
            manifest_items.push(StandardManifestItem {
                id,
                href,
                media_type,
                properties,
            });
        }

        search_pos = item_end;
    }

    // 解析 spine
    let mut spine_itemrefs = Vec::new();
    if let Some(spine_start) = opf_content.find("<spine") {
        let spine_content = &opf_content[spine_start..];
        if let Some(spine_end) = spine_content.find("</spine>") {
            let spine_section = &spine_content[..spine_end];
            let mut itemref_pos = 0;
            while let Some(itemref_start) = spine_section[itemref_pos..].find("<itemref") {
                let itemref_start = itemref_pos + itemref_start;
                let itemref_end = spine_section[itemref_start..].find("/>")
                    .map(|end| itemref_start + end + 2)
                    .unwrap_or(spine_section.len());

                let itemref_content = &spine_section[itemref_start..itemref_end];
                let idref = extract_xml_attr(itemref_content, "idref").unwrap_or_default();
                let linear = extract_xml_attr(itemref_content, "linear").unwrap_or_default();

                if !idref.is_empty() {
                    spine_itemrefs.push(StandardSpineItemref {
                        idref,
                        linear: if linear.is_empty() { None } else { Some(linear == "yes") },
                    });
                }

                itemref_pos = itemref_end;
            }
        }
    }

    let metadata = parse_opf_metadata(&opf_content, &epub_id);
    let manifest = StandardManifest { items: manifest_items };
    let spine = StandardSpine { itemrefs: spine_itemrefs };

    // 加载导航
    let navigation = load_navigation_from_cache(&oebps_path)?;

    // 构建 StandardEpubStructure
    let chapters: Vec<StandardChapter> = manifest.items
        .iter()
        .filter(|item| {
            is_renderable_chapter_item(item)
        })
        .filter_map(|item| {
            let title = find_navigation_title(&navigation, &item.href);

            let original_filename = item.href.rsplit('/').next().unwrap_or(&item.href).to_string();
            let standard_path = format!("OEBPS/{}", item.href);

            let order = spine.itemrefs
                .iter()
                .position(|spine_item| spine_item.idref == item.id)
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

    let styles: Vec<String> = manifest.items
        .iter()
        .filter(|item| item.media_type == "text/css")
        .map(|item| format!("OEBPS/{}", item.href))
        .collect();

    let images: Vec<String> = manifest.items
        .iter()
        .filter(|item| item.media_type.starts_with("image/"))
        .map(|item| format!("OEBPS/{}", item.href))
        .collect();

    let structure = StandardEpubStructure {
        chapters,
        styles,
        images,
        fonts: manifest.items
            .iter()
            .filter(|item| {
                matches!(item.media_type.as_str(),
                    "font/ttf" | "font/otf" | "font/woff" | "font/woff2" |
                    "application/font-ttf" | "application/font-woff" | "application/font-woff2")
            })
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        navigation,
    };

    Ok(RefactoredEpubResult {
        epub_id: epub_id.clone(),
        metadata,
        structure,
        storage_path,
    })
}

/// 从缓存目录加载导航结构
fn load_navigation_from_cache(oebps_path: &Path) -> Result<Vec<NavigationEntry>, String> {
    let nav_path = oebps_path.join("nav.xhtml");
    let ncx_path = oebps_path.join("toc.ncx");

    let toc: Vec<NavigationEntry> = if nav_path.exists() {
        let content = fs::read_to_string(&nav_path)
            .map_err(|e| format!("无法读取 nav.xhtml: {}", e))?;

        // 解析 nav.xhtml
        let mut entries = Vec::new();
        let mut order_counter = 0usize;
        parse_nav_xhtml(&content, 0, &mut entries, &mut order_counter, "");
        entries
    } else if ncx_path.exists() {
        let content = fs::read_to_string(&ncx_path)
            .map_err(|e| format!("无法读取 toc.ncx: {}", e))?;

        // 解析 toc.ncx
        let mut entries = Vec::new();
        let mut order_counter = 0usize;
        parse_toc_ncx(&content, 0, &mut entries, &mut order_counter, "");
        entries
    } else {
        Vec::new()
    };

    Ok(toc)
}

/// 解析 nav.xhtml
fn parse_nav_xhtml(
    content: &str,
    level: usize,
    entries: &mut Vec<NavigationEntry>,
    order_counter: &mut usize,
    _base_path: &str,
) {
    // 查找所有 <li> 元素
    let mut search_pos = 0;
    while let Some(li_start) = content[search_pos..].find("<li") {
        let li_start = search_pos + li_start;
        let li_end = match find_matching_close_tag(&content[li_start..], "<li", "</li>") {
            Some(end) => li_start + end,
            None => {
                search_pos = li_start + 3;
                continue;
            }
        };

        let li_content = &content[li_start..li_end];

        if let Some(a_start) = li_content.find("<a ") {
            if let Some(a_end) = li_content[a_start..].find("</a>") {
                let a_content = &li_content[a_start..a_start + a_end + 4];

                let href = extract_html_attr(a_content, "href").unwrap_or_default();
                let label = extract_text_content(a_content).unwrap_or_else(|| "Untitled".to_string());

                if !href.is_empty() || !label.is_empty() {
                    let order = *order_counter;
                    *order_counter += 1;

                    let mut children = Vec::new();
                    if let Some(ol_start) = li_content.find("<ol") {
                        let ol_end = find_matching_close_tag(&li_content[ol_start..], "<ol", "</ol>")
                            .map(|end| ol_start + end)
                            .unwrap_or(li_content.len());
                        let ol_content = &li_content[ol_start..ol_end];
                        parse_nav_xhtml(ol_content, level + 1, &mut children, order_counter, "");
                    }

                    entries.push(NavigationEntry {
                        id: format!("nav-{}", order),
                        label,
                        content_src: href,
                        level,
                        children,
                    });
                }
            }
        }

        search_pos = li_end;
    }
}

/// 解析 toc.ncx
fn parse_toc_ncx(
    content: &str,
    level: usize,
    entries: &mut Vec<NavigationEntry>,
    order_counter: &mut usize,
    _base_path: &str,
) {
    let nav_map_start = match content.find("<navMap") {
        Some(pos) => pos,
        None => return,
    };

    let nav_map_content = &content[nav_map_start..];
    let mut search_pos = 0;

    while let Some(point_start) = nav_map_content[search_pos..].find("<navPoint") {
        let point_start = search_pos + point_start;
        let point_end = find_matching_close_tag(&nav_map_content[point_start..], "<navPoint", "</navPoint>")
            .map(|end| point_start + end)
            .unwrap_or(nav_map_content.len());

        let nav_point_content = &nav_map_content[point_start..point_end];

        let id = extract_xml_attr(nav_point_content, "id").unwrap_or_else(|| format!("navpoint-{}", *order_counter));
        let label = extract_nav_label(nav_point_content).unwrap_or_else(|| "Untitled".to_string());
        let content_src = extract_ncx_content_src(nav_point_content).unwrap_or_default();

        let mut children = Vec::new();
        parse_toc_ncx(nav_point_content, level + 1, &mut children, order_counter, "");

        let order = *order_counter;
        *order_counter += 1;

        entries.push(NavigationEntry {
            id,
            label,
            content_src,
            level,
            children,
        });

        search_pos = point_end;
    }
}

/// 查找匹配的闭合标签
fn find_matching_close_tag(content: &str, open_tag: &str, close_tag: &str) -> Option<usize> {
    let mut depth = 0;
    let mut search_pos = 0;
    let open_lower = open_tag.to_lowercase();
    let close_lower = close_tag.to_lowercase();

    while search_pos < content.len() {
        if let Some(c) = content[search_pos..].chars().next() {
            let char_len = c.len_utf8();
            let lower = content[search_pos..].to_lowercase();

            if lower.starts_with(&open_lower) {
                depth += 1;
                search_pos += open_tag.len();
            } else if lower.starts_with(&close_lower) {
                depth -= 1;
                if depth == 0 {
                    return Some(search_pos + close_tag.len());
                }
                search_pos += close_tag.len();
            } else {
                search_pos += char_len;
            }
        } else {
            break;
        }
    }

    None
}

/// 提取 XML 属性
fn extract_xml_attr(content: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = content.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = content[value_start..].find('"') {
            return Some(content[value_start..value_start + end].to_string());
        }
    }
    None
}

/// 提取 HTML 属性
fn extract_html_attr(content: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = content.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = content[value_start..].find('"') {
            return Some(content[value_start..value_start + end].to_string());
        }
    }
    None
}

/// 提取文本内容
fn extract_text_content(content: &str) -> Option<String> {
    if let Some(start) = content.find('>') {
        let text = &content[start + 1..];
        if let Some(end) = text.find("</a>") {
            let text = text[..end].trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// 提取 nav label
fn extract_nav_label(content: &str) -> Option<String> {
    if let Some(text_start) = content.find("<text>") {
        if let Some(text_end) = content[text_start..].find("</text>") {
            return Some(content[text_start + 6..text_start + text_end].trim().to_string());
        }
    }
    None
}

/// 提取 NCX content src
fn extract_ncx_content_src(content: &str) -> Option<String> {
    if let Some(content_start) = content.find("<content src=\"") {
        let value_start = content_start + 14;
        if let Some(quote_end) = content[value_start..].find('"') {
            return Some(content[value_start..value_start + quote_end].to_string());
        }
    }
    None
}

fn is_renderable_chapter_item(item: &StandardManifestItem) -> bool {
    (item.media_type == "application/xhtml+xml" || item.media_type == "text/html")
        && !item
            .properties
            .as_ref()
            .map(|props| props.iter().any(|prop| prop == "nav"))
            .unwrap_or(false)
}

fn find_navigation_title(entries: &[NavigationEntry], href: &str) -> Option<String> {
    let normalized_href = href.split('#').next().unwrap_or(href);

    for entry in entries {
        if entry.content_src.split('#').next().unwrap_or(&entry.content_src) == normalized_href {
            return Some(entry.label.clone());
        }

        if let Some(title) = find_navigation_title(&entry.children, href) {
            return Some(title);
        }
    }

    None
}

fn parse_opf_metadata(opf_content: &str, fallback_identifier: &str) -> EpubMetadata {
    let identifier = extract_xml_tag(opf_content, "dc:identifier")
        .unwrap_or_else(|| fallback_identifier.to_string());

    EpubMetadata {
        title: extract_xml_tag(opf_content, "dc:title")
            .unwrap_or_else(|| fallback_identifier.to_string()),
        author: extract_xml_tag(opf_content, "dc:creator"),
        language: extract_xml_tag(opf_content, "dc:language"),
        identifier,
    }
}

fn extract_xml_tag(content: &str, tag_name: &str) -> Option<String> {
    let close_tag = format!("</{}>", tag_name);
    let open_tag_start = format!("<{}", tag_name);
    let start = content.find(&open_tag_start)?;
    let open_end = content[start..].find('>')? + start;
    let after_start = open_end + 1;
    let end = content[after_start..].find(&close_tag)? + after_start;
    let value = content[after_start..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_opf_metadata_reads_original_values() {
        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package>
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">book-123</dc:identifier>
    <dc:title>Example Book</dc:title>
    <dc:creator>Author</dc:creator>
    <dc:language>zh-CN</dc:language>
  </metadata>
</package>"#;

        let metadata = parse_opf_metadata(opf, "fallback-id");
        assert_eq!(metadata.identifier, "book-123");
        assert_eq!(metadata.title, "Example Book");
        assert_eq!(metadata.author.as_deref(), Some("Author"));
        assert_eq!(metadata.language.as_deref(), Some("zh-CN"));
    }
}
