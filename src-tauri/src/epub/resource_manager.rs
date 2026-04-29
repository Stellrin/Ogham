use super::epub_path;
use super::models::*;
use base64::Engine;
use regex::Regex;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::LazyLock;
use zip::ZipArchive;

static ATTRIBUTE_REFERENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(src|href|xlink:href)="([^"]+)""#).expect("valid EPUB attribute reference regex")
});

static CSS_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"url\(\s*(['"]?)([^'")]+)(['"]?)\s*\)"#).expect("valid CSS url reference regex")
});

/// 从原始 EPUB 复制和整理文件到标准目录
pub fn copy_and_organize_files(
    source_path: &str,
    storage_path: &str,
    analysis: &EpubStructureAnalysis,
) -> Result<(), RefactorError> {
    // 打开源 EPUB 文件
    let file = File::open(source_path)
        .map_err(|e| RefactorError::IoError(format!("无法打开源文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| RefactorError::ParseError(format!("无法打开 ZIP 归档: {}", e)))?;

    // 复制章节文件
    for chapter in &analysis.chapters {
        extract_and_copy_file(
            &mut archive,
            &chapter.original_path,
            storage_path,
            &chapter.standard_path,
        )?;
    }

    // 复制样式文件
    for style in &analysis.resources.styles {
        extract_and_copy_file(
            &mut archive,
            &style.original_path,
            storage_path,
            &style.standard_path,
        )?;
    }

    // 复制图片文件
    for image in &analysis.resources.images {
        extract_and_copy_file(
            &mut archive,
            &image.original_path,
            storage_path,
            &image.standard_path,
        )?;
    }

    // 复制字体文件
    for font in &analysis.resources.fonts {
        extract_and_copy_file(
            &mut archive,
            &font.original_path,
            storage_path,
            &font.standard_path,
        )?;
    }

    // 复制其他文件
    for misc in &analysis.resources.misc {
        extract_and_copy_file(
            &mut archive,
            &misc.original_path,
            storage_path,
            &misc.standard_path,
        )?;
    }

    // 复制封面页。若封面页已经作为 spine 章节复制，这里覆盖到同一路径，不改变内容。
    if let Some(ref cover_path) = analysis.special_files.cover {
        if let Some(standard_path) = analysis.file_map.original_to_standard.get(cover_path) {
            extract_and_copy_file(&mut archive, cover_path, storage_path, standard_path)?;
        }
    }

    // 复制 nav.xhtml (EPUB 3.0 导航文件)
    if let Some(ref nav_path) = analysis.special_files.nav {
        let standard_path = "OEBPS/nav.xhtml";
        extract_and_copy_file(&mut archive, nav_path, storage_path, standard_path)?;
    }

    // 复制 toc.ncx (EPUB 2.0 目录文件)
    if let Some(ref ncx_path) = analysis.special_files.ncx {
        let standard_path = "OEBPS/toc.ncx";
        extract_and_copy_file(&mut archive, ncx_path, storage_path, standard_path)?;
    }

    Ok(())
}

/// 从 ZIP 中提取并复制文件
fn extract_and_copy_file(
    archive: &mut ZipArchive<File>,
    original_path: &str,
    storage_path: &str,
    standard_path: &str,
) -> Result<(), RefactorError> {
    // 尝试从 ZIP 中提取文件
    let mut source_file = match archive.by_name(original_path) {
        Ok(file) => file,
        Err(_) => {
            // 文件不存在，跳过
            return Ok(());
        }
    };

    // 读取文件内容
    let mut buffer = Vec::new();
    source_file
        .read_to_end(&mut buffer)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {}: {}", original_path, e)))?;

    // 创建目标路径
    let target_path = Path::new(storage_path).join(standard_path);

    // 确保父目录存在
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| RefactorError::IoError(format!("无法创建目录 {:?}: {}", parent, e)))?;
    }

    // 写入文件
    let mut target_file = File::create(&target_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建文件 {:?}: {}", target_path, e)))?;

    target_file
        .write_all(&buffer)
        .map_err(|e| RefactorError::IoError(format!("无法写入文件 {:?}: {}", target_path, e)))?;

    Ok(())
}

/// 更新文件中的资源引用
pub fn update_resource_references(
    storage_path: &str,
    analysis: &EpubStructureAnalysis,
) -> Result<(), RefactorError> {
    // 更新章节文件中的引用
    for chapter in &analysis.chapters {
        update_file_references(
            storage_path,
            &chapter.original_path,
            &chapter.standard_path,
            &analysis.file_map,
        )?;
    }

    // 更新样式文件中的引用
    for style in &analysis.resources.styles {
        update_file_references(
            storage_path,
            &style.original_path,
            &style.standard_path,
            &analysis.file_map,
        )?;
    }

    if let Some(ref cover_path) = analysis.special_files.cover {
        let cover_already_updated = analysis
            .chapters
            .iter()
            .any(|chapter| chapter.original_path == *cover_path);

        if !cover_already_updated {
            if let Some(standard_path) = analysis.file_map.original_to_standard.get(cover_path) {
                if epub_path::is_text_like_file(standard_path) {
                    update_file_references(
                        storage_path,
                        cover_path,
                        standard_path,
                        &analysis.file_map,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// 更新单个文件中的资源引用
fn update_file_references(
    storage_path: &str,
    original_path: &str,
    standard_path: &str,
    file_map: &FileMap,
) -> Result<(), RefactorError> {
    let full_path = Path::new(storage_path).join(standard_path);

    // 读取文件内容
    let content = fs::read_to_string(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", full_path, e)))?;

    // 获取原始文件所在目录（用于解析引用）
    let original_dir = epub_path::parent(original_path);

    // 获取重构后文件所在目录（用于计算新相对路径）
    let standard_dir = epub_path::parent(standard_path);

    // 重写资源路径
    let rewritten = rewrite_resource_paths(&content, &original_dir, &standard_dir, file_map);

    if rewritten != content {
        fs::write(&full_path, rewritten)
            .map_err(|e| RefactorError::IoError(format!("无法写入文件 {:?}: {}", full_path, e)))?;
    }

    Ok(())
}

/// 重写资源路径
fn rewrite_resource_paths(
    content: &str,
    original_dir: &str,
    standard_dir: &str,
    file_map: &FileMap,
) -> String {
    let result = ATTRIBUTE_REFERENCE_RE.replace_all(content, |caps: &regex::Captures| {
        let attr_name = &caps[1];
        let original_ref = &caps[2];

        if let Some(rewritten_ref) =
            rewrite_single_reference(original_ref, original_dir, standard_dir, file_map)
        {
            format!(r#"{}="{}""#, attr_name, rewritten_ref)
        } else {
            format!(r#"{}="{}""#, attr_name, original_ref)
        }
    });

    rewrite_css_url_paths(&result, original_dir, standard_dir, file_map)
}

fn rewrite_css_url_paths(
    content: &str,
    original_dir: &str,
    standard_dir: &str,
    file_map: &FileMap,
) -> String {
    CSS_URL_RE
        .replace_all(content, |caps: &regex::Captures| {
            let open_quote = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let original_ref = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let close_quote = caps.get(3).map(|m| m.as_str()).unwrap_or(open_quote);
            let quote = if !open_quote.is_empty() {
                open_quote
            } else {
                close_quote
            };

            if let Some(rewritten_ref) =
                rewrite_single_reference(original_ref, original_dir, standard_dir, file_map)
            {
                format!("url({}{}{})", quote, rewritten_ref, quote)
            } else {
                caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        })
        .to_string()
}

fn rewrite_single_reference(
    original_ref: &str,
    original_dir: &str,
    standard_dir: &str,
    file_map: &FileMap,
) -> Option<String> {
    let trimmed_ref = original_ref.trim();
    if epub_path::should_skip_reference(trimmed_ref) {
        return None;
    }

    let (path_without_suffix, suffix) = epub_path::split_reference_suffix(trimmed_ref);
    if path_without_suffix.is_empty() {
        return None;
    }

    // 使用原始目录解析相对引用，得到原始完整路径
    let original_full_path = epub_path::resolve_relative(original_dir, path_without_suffix);

    // 在 file_map 中查找对应的标准路径；如果原始文档使用了根相对路径，也直接尝试原引用。
    let standard_target_path = find_standard_path(&original_full_path, file_map)
        .or_else(|| find_standard_path(path_without_suffix, file_map))?;

    // 计算从重构后目录到目标标准路径的相对路径，并保留锚点/查询串。
    let relative_path = epub_path::relative_path(standard_dir, &standard_target_path);
    Some(format!("{}{}", relative_path, suffix))
}

/// 查找标准路径（尝试多种路径格式）
fn find_standard_path(path: &str, file_map: &FileMap) -> Option<String> {
    let normalized_path = epub_path::normalize(path);

    // 首先直接查找
    if let Some(std_path) = file_map.original_to_standard.get(path) {
        return Some(std_path.clone());
    }
    if let Some(std_path) = file_map.original_to_standard.get(&normalized_path) {
        return Some(std_path.clone());
    }

    // 尝试去掉 OEBPS/ 前缀
    if let Some(rest) = normalized_path.strip_prefix("OEBPS/") {
        if let Some(std_path) = file_map.original_to_standard.get(rest) {
            return Some(std_path.clone());
        }
    }

    // 尝试添加 OEBPS/ 前缀
    let with_prefix = format!("OEBPS/{}", normalized_path.trim_start_matches('/'));
    if let Some(std_path) = file_map.original_to_standard.get(&with_prefix) {
        return Some(std_path.clone());
    }

    let folded_path = epub_path::casefold(&normalized_path);
    if let Some((_, std_path)) = file_map
        .original_to_standard
        .iter()
        .find(|(original_path, _)| epub_path::casefold(original_path) == folded_path)
    {
        return Some(std_path.clone());
    }

    // 如果路径本身就是标准路径，直接返回
    if normalized_path.starts_with("OEBPS/") {
        return Some(normalized_path);
    }

    None
}

/// 从重构后的 EPUB 中读取文件内容
pub fn read_refactored_file(storage_path: &str, file_path: &str) -> Result<String, RefactorError> {
    let full_path = Path::new(storage_path).join(file_path);

    fs::read_to_string(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", full_path, e)))
}

/// 从重构后的 EPUB 中提取章节内容（含嵌入的 Base64 资源）
pub fn extract_refactored_chapter_content(
    storage_path: &str,
    chapter_path: &str,
) -> Result<ChapterContent, RefactorError> {
    // 读取章节 HTML
    let html_content = read_refactored_file(storage_path, chapter_path)?;

    // 提取资源并转换为 Base64
    let resources = extract_resources_from_content(storage_path, &html_content, chapter_path)?;

    // 重写 HTML 中的资源路径为 data URI
    let processed_html = rewrite_resource_urls(&html_content, &resources);

    Ok(ChapterContent {
        html: processed_html,
        resources,
    })
}

/// 从内容中提取引用的资源
fn extract_resources_from_content(
    storage_path: &str,
    html: &str,
    chapter_path: &str,
) -> Result<std::collections::HashMap<String, ResourceData>, RefactorError> {
    let mut resources = std::collections::HashMap::new();

    // 获取章节所在目录
    let chapter_dir = epub_path::parent(chapter_path);

    // 正则表达式匹配 src=""、href="" 和 xlink:href="" 属性
    let src_regex = Regex::new(r#"(src|href|xlink:href)="([^"]+)""#)
        .map_err(|e| RefactorError::ParseError(e.to_string()))?;

    for captures in src_regex.captures_iter(html) {
        let resource_path = &captures[2];

        // 跳过已经是 data: URI 的
        if epub_path::should_skip_reference(resource_path) {
            continue;
        }

        // 跳过 CSS 样式表（暂时简化处理）
        if resource_path.ends_with(".css") {
            continue;
        }

        let (resource_path_part, _) = epub_path::split_reference_suffix(resource_path);
        let full_path = if resource_path_part.starts_with("OEBPS/") {
            epub_path::normalize(resource_path_part)
        } else if ["Text/", "Styles/", "Images/", "Fonts/", "Misc/"]
            .iter()
            .any(|prefix| resource_path_part.starts_with(prefix))
        {
            epub_path::manifest_path(resource_path_part)
        } else {
            epub_path::resolve_relative(&chapter_dir, resource_path_part)
        };

        // 如果已经提取过这个资源，跳过
        if resources.contains_key(&captures[2]) {
            continue;
        }

        // 尝试从文件系统读取资源
        if let Ok(resource_data) = extract_single_resource_from_file(storage_path, &full_path) {
            resources.insert(captures[2].to_string(), resource_data);
        }
    }

    Ok(resources)
}

/// 从文件系统中提取单个资源并转为 Base64
fn extract_single_resource_from_file(
    storage_path: &str,
    path: &str,
) -> Result<ResourceData, RefactorError> {
    let full_path = Path::new(storage_path).join(path);

    let buffer = fs::read(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取资源 {:?}: {}", full_path, e)))?;

    // 根据 MIME 类型编码
    let mime_type = guess_mime_type(path);
    let base64_engine = base64::engine::general_purpose::STANDARD;
    let data = base64_engine.encode(&buffer);

    Ok(ResourceData { mime_type, data })
}

/// 猜测文件的 MIME 类型
fn guess_mime_type(path: &str) -> String {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if path_lower.ends_with(".png") {
        "image/png".to_string()
    } else if path_lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if path_lower.ends_with(".svg") {
        "image/svg+xml".to_string()
    } else if path_lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if path_lower.ends_with(".css") {
        "text/css".to_string()
    } else if path_lower.ends_with(".ttf") {
        "font/ttf".to_string()
    } else if path_lower.ends_with(".woff") {
        "font/woff".to_string()
    } else if path_lower.ends_with(".woff2") {
        "font/woff2".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// 重写 HTML 中的资源路径为 data URI
fn rewrite_resource_urls(
    html: &str,
    resources: &std::collections::HashMap<String, ResourceData>,
) -> String {
    let mut result = html.to_string();

    for (original_path, resource_data) in resources {
        // 只处理图片类型的资源（image/*, svg+xml）
        let is_image = resource_data.mime_type.starts_with("image/")
            || resource_data.mime_type.contains("svg");

        if !is_image {
            continue; // 跳过非图片资源
        }

        let data_uri = format!(
            "data:{};base64,{}",
            resource_data.mime_type, resource_data.data
        );

        // 替换 src="" （用于 <img> 等标签）
        result = result.replace(
            &format!("src=\"{}\"", original_path),
            &format!("src=\"{}\"", &data_uri),
        );

        // 替换 xlink:href=""（用于 SVG <image> 标签）
        result = result.replace(
            &format!("xlink:href=\"{}\"", original_path),
            &format!("xlink:href=\"{}\"", &data_uri),
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(
            epub_path::resolve_relative("OEBPS/Text", "../Images/cover.jpg"),
            "OEBPS/Images/cover.jpg"
        );

        assert_eq!(
            epub_path::resolve_relative("OEBPS/Text", "chapter2.xhtml"),
            "OEBPS/Text/chapter2.xhtml"
        );
    }

    #[test]
    fn test_calculate_relative_path() {
        assert_eq!(
            epub_path::relative_path("OEBPS/Text", "OEBPS/Images/cover.jpg"),
            "../Images/cover.jpg"
        );

        assert_eq!(
            epub_path::relative_path("OEBPS/Text", "OEBPS/Styles/style.css"),
            "../Styles/style.css"
        );

        assert_eq!(
            epub_path::relative_path("OEBPS", "OEBPS/Text/chapter1.xhtml"),
            "Text/chapter1.xhtml"
        );
    }

    #[test]
    fn test_rewrite_resource_paths() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        // 原始路径：OEBPS/Images/cover.jpg
        // 标准路径：OEBPS/Images/cover.jpg (相同)
        file_map.original_to_standard.insert(
            "OEBPS/Images/cover.jpg".to_string(),
            "OEBPS/Images/cover.jpg".to_string(),
        );

        // 测试：文件从 OEBPS/Text 移动到 OEBPS/Text
        // 原始引用：../Images/cover.jpg (相对于 OEBPS/Text)
        let html = r#"<html><body><img src="../Images/cover.jpg" /></body></html>"#;
        let result = rewrite_resource_paths(html, "OEBPS/Text", "OEBPS/Text", &file_map);

        assert!(result.contains("src=\"../Images/cover.jpg\""));
    }

    #[test]
    fn test_rewrite_same_dir_image_ref() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        // 原始：cover.xhtml 在 OEBPS/，引用 cover.png (同目录)
        // 重构后：cover.xhtml 在 OEBPS/Text/，图片在 OEBPS/Images/
        file_map.original_to_standard.insert(
            "cover.png".to_string(),
            "OEBPS/Images/cover.png".to_string(),
        );

        let html = r#"<html><body><svg><image xlink:href="cover.png"/></svg></body></html>"#;
        // 原始目录：OEBPS (空字符串表示根目录)
        // 重构后目录：OEBPS/Text
        let result = rewrite_resource_paths(html, "", "OEBPS/Text", &file_map);

        // 应该更新为 ../Images/cover.png
        assert!(result.contains("xlink:href=\"../Images/cover.png\""));
    }

    #[test]
    fn test_rewrite_chapter_href_preserves_anchor() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        file_map.original_to_standard.insert(
            "OEBPS/Text/chapter2.xhtml".to_string(),
            "OEBPS/Text/chapter2.xhtml".to_string(),
        );

        let html = r#"<a href="chapter2.xhtml#section-2">Next</a>"#;
        let result = rewrite_resource_paths(html, "OEBPS/Text", "OEBPS/Text", &file_map);

        assert!(result.contains("href=\"chapter2.xhtml#section-2\""));
    }

    #[test]
    fn test_rewrite_css_url_paths() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        file_map.original_to_standard.insert(
            "OEBPS/images/bg.png".to_string(),
            "OEBPS/Images/bg.png".to_string(),
        );

        let css = r#".cover { background-image: url("../images/bg.png"); }"#;
        let result = rewrite_resource_paths(css, "OEBPS/css", "OEBPS/Styles", &file_map);

        assert!(result.contains("url(\"../Images/bg.png\")"));
    }
}
