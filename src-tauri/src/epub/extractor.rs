use super::models::*;
use super::parser::{EpubParseError, get_opf_base_path};
use base64::Engine;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

/// 提取章节内容并将资源转为 Base64
pub fn extract_chapter_content(
    epub_path: &str,
    chapter_path: &str,
    _manifest: &Manifest,
) -> Result<ChapterContent, EpubParseError> {
    let file =
        File::open(epub_path).map_err(|e| EpubParseError::InvalidZip(e.to_string()))?;
    let mut archive = ZipArchive::new(file)?;

    // 读取章节 HTML
    let html_content = {
        let mut chapter_file = archive
            .by_name(chapter_path)
            .map_err(|_| {
                println!("[ERROR] Failed to find chapter '{}' in ZIP archive", chapter_path);
                EpubParseError::MissingFile(chapter_path.to_string())
            })?;

        let mut content = String::new();
        chapter_file
            .read_to_string(&mut content)
            .map_err(|e| EpubParseError::XmlError(e.to_string()))?;
        content
    }; // chapter_file 在这里被丢弃，释放对 archive 的借用

    // 提取所有资源
    let resources = extract_resources(&mut archive, &html_content, chapter_path)?;

    // 重写 HTML 中的资源路径为 data URI
    let processed_html = rewrite_resource_urls(&html_content, &resources);

    Ok(ChapterContent {
        html: processed_html,
        resources,
    })
}

/// 从章节路径推断 OPF 基础路径
fn get_opf_base_path_from_path(chapter_path: &str) -> String {
    // 通常章节在 OEBPS/Text/ 或类似目录下
    // 我们需要找到 OEBPS 或其他基础目录
    let parts: Vec<&str> = chapter_path.split('/').collect();
    if parts.len() >= 2 {
        // 返回到 OEBPS 或类似目录
        parts[0..parts.len() - 1].join("/")
    } else {
        String::new()
    }
}

/// 提取 HTML 中引用的所有资源
fn extract_resources(
    archive: &mut ZipArchive<File>,
    html: &str,
    chapter_path: &str,
) -> Result<std::collections::HashMap<String, ResourceData>, EpubParseError> {
    let mut resources = std::collections::HashMap::new();

    // 获取章节所在目录
    let chapter_dir = if let Some(last_slash) = chapter_path.rfind('/') {
        &chapter_path[..last_slash]
    } else {
        ""
    };

    // 正则表达式匹配 src=""、href="" 和 xlink:href="" 属性
    let src_regex =
        Regex::new(r#"(src|href|xlink:href)="([^"]+)""#).map_err(|e| EpubParseError::XmlError(e.to_string()))?;

    for captures in src_regex.captures_iter(html) {
        let resource_path = &captures[2];

        // 跳过已经是 data: URI 的
        if resource_path.starts_with("data:") {
            continue;
        }

        // 跳过外部链接和锚点
        if resource_path.starts_with("http://")
            || resource_path.starts_with("https://")
            || resource_path.starts_with('#')
        {
            continue;
        }

        // 跳过 CSS 样式表（暂时简化处理）
        if resource_path.ends_with(".css") {
            continue;
        }

        // 解析相对路径
        let full_path = resolve_path(chapter_dir, resource_path);

        // 如果已经提取过这个资源，跳过
        if resources.contains_key(&captures[2]) {
            continue;
        }

        // 尝试从 ZIP 中提取资源
        if let Ok(resource_data) = extract_single_resource(archive, &full_path) {
            resources.insert(captures[2].to_string(), resource_data);
        }
    }

    Ok(resources)
}

/// 从 ZIP 中提取单个资源并转为 Base64
fn extract_single_resource(
    archive: &mut ZipArchive<File>,
    path: &str,
) -> Result<ResourceData, EpubParseError> {
    let mut resource_file = archive
        .by_name(path)
        .map_err(|_| EpubParseError::MissingFile(path.to_string()))?;

    let mut buffer = Vec::new();
    resource_file
        .read_to_end(&mut buffer)
        .map_err(|e| EpubParseError::XmlError(e.to_string()))?;

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

/// 解析相对路径
fn resolve_path(base: &str, relative: &str) -> String {
    // 处理 ../ 路径
    let mut base_parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();

    for part in &path_parts {
        if *part == ".." {
            base_parts.pop();
        } else if !part.starts_with('.') {
            base_parts.push(part);
        }
    }

    if base_parts.is_empty() {
        path_parts.join("/")
    } else {
        base_parts.join("/")
    }
}

/// 重写 HTML 中的资源路径为 data URI
/// 只替换图片的链接，不替换 <a> 标签的 href
fn rewrite_resource_urls(
    html: &str,
    resources: &std::collections::HashMap<String, ResourceData>,
) -> String {
    let mut result = html.to_string();

    for (original_path, resource_data) in resources {
        // 只处理图片类型的资源（image/*, svg+xml）
        let is_image = resource_data.mime_type.starts_with("image/") ||
                       resource_data.mime_type.contains("svg");

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

/// 将解析的 EPUB 转换为前端需要的结构格式
pub fn convert_to_structure_result(
    parsed: &ParsedEpub,
    opf_path: &str,
) -> EpubStructureResult {
    let oebps_path = get_opf_base_path(opf_path);

    // 从 manifest 中提取各种文件
    let mut chapters = Vec::new();
    let mut styles = Vec::new();
    let mut images = Vec::new();
    let mut toc_ncx = None;
    let mut nav_xhtml = None;

    for (order, itemref) in parsed.spine.itemrefs.iter().enumerate() {
        if let Some(item) = parsed.manifest.items.get(&itemref.idref) {
            // 提取文件名作为章节名
            let name = item
                .href
                .rsplit('/')
                .next()
                .unwrap_or(&item.href)
                .to_string();

            // item.href 已经是 epub 库返回的完整 ZIP 路径（来自 resource.path）
            // 不需要再添加 oebps_path 前缀
            let path = item.href.clone();

            chapters.push(ChapterInfo {
                id: item.id.clone(),
                path,
                name,
                order,
            });
        }
    }

    // 遍历 manifest 提取样式和图片
    for (_id, item) in &parsed.manifest.items {
        if item.media_type.starts_with("text/css") {
            // 存储完整路径
            styles.push(item.href.clone());
        } else if item.media_type.starts_with("image/") {
            // 存储完整路径（与章节保持一致）
            images.push(item.href.clone());
        } else if item.media_type == "application/x-dtbncx+xml" {
            toc_ncx = Some(item.href.clone());
        } else if item.properties.as_ref().map_or(false, |p| p.contains(&"nav".to_string())) {
            nav_xhtml = Some(item.href.clone());
        }
    }

    EpubStructureResult {
        metadata: parsed.metadata.clone(),
        oebps_path,
        content_opf: opf_path.to_string(),
        toc_ncx,
        nav_xhtml,
        chapters,
        styles,
        images,
    }
}
