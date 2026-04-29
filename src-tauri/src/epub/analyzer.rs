use super::epub_path;
use super::models::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;
use zip::ZipArchive;

static ATTRIBUTE_REFERENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(src|href|xlink:href)="([^"]+)""#).expect("valid EPUB attribute reference regex")
});

static CSS_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"url\(\s*(['"]?)([^'")]+)(['"]?)\s*\)"#).expect("valid CSS url reference regex")
});

/// 分析 EPUB 结构
pub fn analyze_epub_structure(
    parsed: &ParsedEpub,
    opf_path: &str,
    source_path: &str,
) -> Result<EpubStructureAnalysis, RefactorError> {
    // 识别章节文件（从 spine）
    let mut chapters = identify_chapters(parsed)?;

    // 分类资源文件
    let mut resources = categorize_resources(parsed, &chapters)?;

    discover_unmanifested_resources(source_path, opf_path, &chapters, &mut resources)?;

    // 识别特殊文件
    let special_files = identify_special_files(parsed)?;

    // 标准目录落到真实文件系统上，必须提前处理大小写不敏感平台上的路径冲突
    allocate_unique_standard_paths(&mut chapters, &mut resources);

    // 构建文件映射
    let file_map = build_file_map(&chapters, &resources, &special_files);

    Ok(EpubStructureAnalysis {
        chapters,
        resources,
        special_files,
        file_map,
    })
}

/// 从 spine 识别章节文件
fn identify_chapters(parsed: &ParsedEpub) -> Result<Vec<ExtendedChapterInfo>, RefactorError> {
    let mut chapters = Vec::new();
    let mut seen_ids = HashSet::new();
    let toc_titles = collect_toc_titles(parsed);

    for (order, itemref) in parsed.spine.itemrefs.iter().enumerate() {
        // 通过 idref 查找 manifest 中的项目
        let manifest_item = parsed.manifest.items.get(&itemref.idref).ok_or_else(|| {
            RefactorError::InvalidStructure(format!(
                "Spine 引用的项目 {} 在 manifest 中不存在",
                itemref.idref
            ))
        })?;

        // 只处理 XHTML/HTML 文件作为章节
        if !is_chapter_media_type(&manifest_item.media_type) {
            continue;
        }

        // 跳过导航文件；封面页如果在 spine 中，仍按阅读顺序作为 Text/ 文件管理
        if is_navigation_file(&manifest_item) {
            continue;
        }

        // 检查重复 ID
        if seen_ids.contains(&manifest_item.id) {
            continue;
        }
        seen_ids.insert(manifest_item.id.clone());

        // 提取文件名
        let standard_path = epub_path::standard_path("Text", &manifest_item.href, &["Text"]);

        let title = toc_titles
            .get(&epub_path::normalize(&manifest_item.href))
            .cloned();

        chapters.push(ExtendedChapterInfo {
            id: manifest_item.id.clone(),
            original_path: manifest_item.href.clone(),
            standard_path,
            title,
            order,
            media_type: manifest_item.media_type.clone(),
        });
    }

    if chapters.is_empty() {
        return Err(RefactorError::InvalidStructure(
            "未找到任何章节文件".to_string(),
        ));
    }

    Ok(chapters)
}

/// 检查是否是章节媒体类型
fn is_chapter_media_type(media_type: &str) -> bool {
    matches!(
        media_type,
        "application/xhtml+xml" | "text/html" | "application/xml"
    )
}

/// 检查是否是特殊文件
fn is_special_file(item: &ManifestItem) -> bool {
    item.properties
        .as_ref()
        .map(|props| {
            props.iter().any(|p| p == "nav")
                || props.iter().any(|p| p == "cover") && is_chapter_media_type(&item.media_type)
        })
        .unwrap_or(false)
        || item.media_type == "application/x-dtbncx+xml"
}

fn has_property(item: &ManifestItem, property: &str) -> bool {
    item.properties
        .as_ref()
        .map(|props| props.iter().any(|p| p == property))
        .unwrap_or(false)
}

fn is_navigation_file(item: &ManifestItem) -> bool {
    has_property(item, "nav")
}

fn collect_toc_titles(parsed: &ParsedEpub) -> HashMap<String, String> {
    let mut titles = HashMap::new();

    if let Some(toc) = parsed.table_of_contents.as_ref() {
        collect_toc_titles_recursive(&toc.nav_points, &mut titles);
    }

    titles
}

fn collect_toc_titles_recursive(nav_points: &[NavPoint], titles: &mut HashMap<String, String>) {
    for nav_point in nav_points {
        let normalized_href = epub_path::normalize(&nav_point.content_src);
        if !normalized_href.is_empty() {
            titles
                .entry(normalized_href)
                .or_insert_with(|| nav_point.label.clone());
        }

        collect_toc_titles_recursive(&nav_point.children, titles);
    }
}

/// 分类资源文件
fn categorize_resources(
    parsed: &ParsedEpub,
    chapters: &[ExtendedChapterInfo],
) -> Result<ResourceCollection, RefactorError> {
    let mut styles = Vec::new();
    let mut images = Vec::new();
    let mut fonts = Vec::new();
    let mut misc = Vec::new();

    // 收集所有章节 ID，以便排除
    let chapter_ids: HashSet<&str> = chapters.iter().map(|c| c.id.as_str()).collect();

    for (id, item) in &parsed.manifest.items {
        // 跳过章节文件
        if chapter_ids.contains(id.as_str()) {
            continue;
        }

        // 跳过特殊文件
        if is_special_file(item) {
            continue;
        }

        // 根据媒体类型确定标准路径
        let standard_path = classify_resource_path(&item.media_type, &item.href);

        let resource_file = ResourceFile {
            id: id.clone(),
            original_path: item.href.clone(),
            standard_path,
            media_type: item.media_type.clone(),
        };

        // 根据媒体类型添加到对应集合
        match classify_resource_type(&item.media_type) {
            ResourceType::Style => styles.push(resource_file),
            ResourceType::Image => images.push(resource_file),
            ResourceType::Font => fonts.push(resource_file),
            ResourceType::Misc => misc.push(resource_file),
        }
    }

    // 按文件名排序
    styles.sort_by(|a, b| a.original_path.cmp(&b.original_path));
    images.sort_by(|a, b| a.original_path.cmp(&b.original_path));
    fonts.sort_by(|a, b| a.original_path.cmp(&b.original_path));
    misc.sort_by(|a, b| a.original_path.cmp(&b.original_path));

    Ok(ResourceCollection {
        styles,
        images,
        fonts,
        misc,
    })
}

fn discover_unmanifested_resources(
    source_path: &str,
    opf_path: &str,
    chapters: &[ExtendedChapterInfo],
    resources: &mut ResourceCollection,
) -> Result<(), RefactorError> {
    let file = File::open(source_path)
        .map_err(|e| RefactorError::IoError(format!("无法打开源文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| RefactorError::ParseError(format!("无法打开 ZIP 归档: {}", e)))?;

    let (available_entries, available_entries_casefold) = collect_zip_entries(&mut archive)?;
    let opf_dir = epub_path::parent(opf_path);
    let mut known_paths = collect_known_original_paths(chapters, resources);
    let mut used_ids = collect_known_ids(chapters, resources);
    let mut scan_queue = collect_scan_queue(chapters, resources);
    let mut scanned_paths = HashSet::new();

    while let Some(source_file_path) = scan_queue.pop() {
        let source_file_path = match find_zip_entry(
            &source_file_path,
            &opf_dir,
            &available_entries,
            &available_entries_casefold,
        ) {
            Some(path) => path,
            None => continue,
        };

        if !scanned_paths.insert(epub_path::casefold(&source_file_path)) {
            continue;
        }

        let Some(content) = read_zip_text(&mut archive, &source_file_path)? else {
            continue;
        };

        let source_dir = epub_path::parent(&source_file_path);
        for reference in collect_resource_references(&content) {
            if epub_path::should_skip_reference(&reference) {
                continue;
            }

            let (path_part, _) = epub_path::split_reference_suffix(&reference);
            if path_part.trim().is_empty() {
                continue;
            }

            let resolved_path = epub_path::resolve_relative(&source_dir, path_part);
            let Some(actual_path) = find_zip_entry(
                &resolved_path,
                &opf_dir,
                &available_entries,
                &available_entries_casefold,
            )
            .or_else(|| {
                find_zip_entry(
                    path_part,
                    &opf_dir,
                    &available_entries,
                    &available_entries_casefold,
                )
            }) else {
                continue;
            };

            let known_key = epub_path::casefold(&actual_path);
            if known_paths.contains(&known_key) {
                continue;
            }

            let media_type = infer_media_type(&actual_path);
            let standard_path = classify_resource_path(&media_type, &actual_path);
            let id = make_unique_inferred_id(&actual_path, &mut used_ids);

            let resource_file = ResourceFile {
                id,
                original_path: actual_path.clone(),
                standard_path,
                media_type: media_type.clone(),
            };

            match classify_resource_type(&media_type) {
                ResourceType::Style => resources.styles.push(resource_file),
                ResourceType::Image => resources.images.push(resource_file),
                ResourceType::Font => resources.fonts.push(resource_file),
                ResourceType::Misc => resources.misc.push(resource_file),
            }

            known_paths.insert(known_key);
            if epub_path::is_text_like_file(&actual_path) {
                scan_queue.push(actual_path);
            }
        }
    }

    sort_resources(resources);
    Ok(())
}

fn collect_zip_entries(
    archive: &mut ZipArchive<File>,
) -> Result<(HashMap<String, String>, HashMap<String, String>), RefactorError> {
    let mut entries = HashMap::new();
    let mut entries_casefold = HashMap::new();

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| RefactorError::ParseError(format!("无法读取 ZIP 条目: {}", e)))?;
        if entry.is_dir() {
            continue;
        }

        let actual_path = epub_path::normalize(entry.name());
        entries
            .entry(actual_path.clone())
            .or_insert_with(|| entry.name().replace('\\', "/"));
        entries_casefold
            .entry(epub_path::casefold(&actual_path))
            .or_insert_with(|| entry.name().replace('\\', "/"));
    }

    Ok((entries, entries_casefold))
}

fn collect_known_original_paths(
    chapters: &[ExtendedChapterInfo],
    resources: &ResourceCollection,
) -> HashSet<String> {
    let mut paths = HashSet::new();

    for chapter in chapters {
        paths.insert(epub_path::casefold(&chapter.original_path));
    }

    for resource in resources.styles.iter().chain(
        resources
            .images
            .iter()
            .chain(resources.fonts.iter().chain(resources.misc.iter())),
    ) {
        paths.insert(epub_path::casefold(&resource.original_path));
    }

    paths
}

fn collect_known_ids(
    chapters: &[ExtendedChapterInfo],
    resources: &ResourceCollection,
) -> HashSet<String> {
    let mut ids = HashSet::new();

    for chapter in chapters {
        ids.insert(chapter.id.clone());
    }

    for resource in resources.styles.iter().chain(
        resources
            .images
            .iter()
            .chain(resources.fonts.iter().chain(resources.misc.iter())),
    ) {
        ids.insert(resource.id.clone());
    }

    ids
}

fn collect_scan_queue(
    chapters: &[ExtendedChapterInfo],
    resources: &ResourceCollection,
) -> Vec<String> {
    let mut scan_queue = Vec::new();

    for chapter in chapters.iter().rev() {
        scan_queue.push(chapter.original_path.clone());
    }

    for resource in resources.styles.iter().rev() {
        scan_queue.push(resource.original_path.clone());
    }

    scan_queue
}

fn collect_resource_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    for captures in ATTRIBUTE_REFERENCE_RE.captures_iter(content) {
        if let Some(reference) = captures.get(2) {
            references.push(reference.as_str().trim().to_string());
        }
    }

    for captures in CSS_URL_RE.captures_iter(content) {
        if let Some(reference) = captures.get(2) {
            references.push(reference.as_str().trim().to_string());
        }
    }

    references
}

fn find_zip_entry(
    path: &str,
    opf_dir: &str,
    available_entries: &HashMap<String, String>,
    available_entries_casefold: &HashMap<String, String>,
) -> Option<String> {
    let mut candidates = vec![epub_path::normalize(path)];

    if !opf_dir.is_empty() {
        let normalized = epub_path::normalize(path);
        if !normalized.starts_with(&format!("{}/", opf_dir)) {
            candidates.push(epub_path::normalize(&format!("{}/{}", opf_dir, normalized)));
        }
    }

    for candidate in candidates {
        if let Some(actual_path) = available_entries.get(&candidate) {
            return Some(actual_path.clone());
        }

        if let Some(actual_path) = available_entries_casefold.get(&epub_path::casefold(&candidate))
        {
            return Some(actual_path.clone());
        }
    }

    None
}

fn read_zip_text(
    archive: &mut ZipArchive<File>,
    path: &str,
) -> Result<Option<String>, RefactorError> {
    let mut source_file = match archive.by_name(path) {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };

    let mut buffer = Vec::new();
    source_file
        .read_to_end(&mut buffer)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {}: {}", path, e)))?;

    Ok(String::from_utf8(buffer).ok())
}

fn infer_media_type(path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "css" => "text/css",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "xhtml" | "html" => "application/xhtml+xml",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn make_unique_inferred_id(path: &str, used_ids: &mut HashSet<String>) -> String {
    let filename = epub_path::filename(path);
    let stem = filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(&filename);
    let base = sanitize_resource_id(&format!("inferred_{}", stem));
    let mut candidate = base.clone();
    let mut suffix = 1usize;

    while used_ids.contains(&candidate) {
        candidate = format!("{}_{}", base, suffix);
        suffix += 1;
    }

    used_ids.insert(candidate.clone());
    candidate
}

fn sanitize_resource_id(value: &str) -> String {
    let id = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if id.is_empty() {
        "inferred_item".to_string()
    } else {
        id
    }
}

fn sort_resources(resources: &mut ResourceCollection) {
    resources
        .styles
        .sort_by(|a, b| a.original_path.cmp(&b.original_path));
    resources
        .images
        .sort_by(|a, b| a.original_path.cmp(&b.original_path));
    resources
        .fonts
        .sort_by(|a, b| a.original_path.cmp(&b.original_path));
    resources
        .misc
        .sort_by(|a, b| a.original_path.cmp(&b.original_path));
}

/// 资源类型枚举
enum ResourceType {
    Style,
    Image,
    Font,
    Misc,
}

/// 根据媒体类型分类资源类型
fn classify_resource_type(media_type: &str) -> ResourceType {
    match media_type {
        "text/css" => ResourceType::Style,
        mt if mt.starts_with("image/") => ResourceType::Image,
        "font/ttf"
        | "font/otf"
        | "font/woff"
        | "font/woff2"
        | "application/font-ttf"
        | "application/font-woff"
        | "application/font-woff2" => ResourceType::Font,
        _ => ResourceType::Misc,
    }
}

/// 根据媒体类型分类资源路径
fn classify_resource_path(media_type: &str, original_href: &str) -> String {
    match classify_resource_type(media_type) {
        ResourceType::Style => {
            epub_path::standard_path("Styles", original_href, &["Styles", "Style"])
        }
        ResourceType::Image => {
            epub_path::standard_path("Images", original_href, &["Images", "Image"])
        }
        ResourceType::Font => epub_path::standard_path("Fonts", original_href, &["Fonts", "Font"]),
        ResourceType::Misc => epub_path::standard_path("Misc", original_href, &["Misc"]),
    }
}

/// 识别特殊文件
fn identify_special_files(parsed: &ParsedEpub) -> Result<SpecialFiles, RefactorError> {
    let mut cover = None;
    let mut nav = None;
    let mut ncx = None;

    for (_id, item) in &parsed.manifest.items {
        let properties = item.properties.as_deref().unwrap_or_default();

        // 检查 cover 属性
        if properties.contains(&"cover".to_string()) {
            cover = Some(item.href.clone());
        }

        // 检查 nav 属性
        if properties.contains(&"nav".to_string()) {
            nav = Some(item.href.clone());
        }

        // 检查 NCX 文件（通过媒体类型）
        if item.media_type == "application/x-dtbncx+xml" {
            ncx = Some(item.href.clone());
        }
    }

    // 如果没有找到 nav，尝试通过文件名查找
    if nav.is_none() {
        for (_id, item) in &parsed.manifest.items {
            let filename = epub_path::filename(&item.href);
            if filename == "nav.xhtml" {
                nav = Some(item.href.clone());
                break;
            }
        }
    }

    Ok(SpecialFiles { cover, nav, ncx })
}

/// 构建文件路径映射
fn build_file_map(
    chapters: &[ExtendedChapterInfo],
    resources: &ResourceCollection,
    special_files: &SpecialFiles,
) -> FileMap {
    let mut original_to_standard = HashMap::new();
    let mut standard_to_original = HashMap::new();
    let mut used_standard_paths = HashSet::new();

    // 添加章节映射
    for chapter in chapters {
        original_to_standard.insert(chapter.original_path.clone(), chapter.standard_path.clone());
        standard_to_original.insert(chapter.standard_path.clone(), chapter.original_path.clone());
        used_standard_paths.insert(epub_path::casefold(&chapter.standard_path));
    }

    // 添加资源映射
    for resource in resources.styles.iter().chain(
        resources
            .images
            .iter()
            .chain(resources.fonts.iter().chain(resources.misc.iter())),
    ) {
        original_to_standard.insert(
            resource.original_path.clone(),
            resource.standard_path.clone(),
        );
        standard_to_original.insert(
            resource.standard_path.clone(),
            resource.original_path.clone(),
        );
        used_standard_paths.insert(epub_path::casefold(&resource.standard_path));
    }

    // 添加特殊文件映射
    if let Some(ref cover) = special_files.cover {
        if !original_to_standard.contains_key(cover) {
            let standard_path = epub_path::allocate_unique_path(
                &epub_path::standard_path("Text", cover, &["Text"]),
                &mut used_standard_paths,
            );
            original_to_standard.insert(cover.clone(), standard_path.clone());
            standard_to_original.insert(standard_path, cover.clone());
        }
    }

    if let Some(ref nav) = special_files.nav {
        let standard_path = format!("OEBPS/{}", epub_path::filename(nav));
        original_to_standard.insert(nav.clone(), standard_path.clone());
        standard_to_original.insert(standard_path, nav.clone());
    }

    if let Some(ref ncx) = special_files.ncx {
        let standard_path = format!("OEBPS/{}", epub_path::filename(ncx));
        original_to_standard.insert(ncx.clone(), standard_path.clone());
        standard_to_original.insert(standard_path, ncx.clone());
    }

    FileMap {
        original_to_standard,
        standard_to_original,
    }
}

fn allocate_unique_standard_paths(
    chapters: &mut [ExtendedChapterInfo],
    resources: &mut ResourceCollection,
) {
    let mut used_paths = HashSet::new();

    for chapter in chapters {
        chapter.standard_path =
            epub_path::allocate_unique_path(&chapter.standard_path, &mut used_paths);
    }

    for resource in resources
        .styles
        .iter_mut()
        .chain(resources.images.iter_mut())
        .chain(resources.fonts.iter_mut())
        .chain(resources.misc.iter_mut())
    {
        resource.standard_path =
            epub_path::allocate_unique_path(&resource.standard_path, &mut used_paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_matches_href_instead_of_manifest_id() {
        let parsed = ParsedEpub {
            metadata: EpubMetadata {
                title: "book".to_string(),
                author: None,
                language: None,
                identifier: "id".to_string(),
            },
            manifest: Manifest {
                items: HashMap::new(),
            },
            spine: Spine {
                itemrefs: Vec::new(),
            },
            table_of_contents: Some(Toc {
                nav_points: vec![NavPoint {
                    id: "nav-1".to_string(),
                    label: "Chapter 1".to_string(),
                    content_src: "Text/chapter1.xhtml#frag".to_string(),
                    children: vec![],
                }],
            }),
        };

        let titles = collect_toc_titles(&parsed);
        assert_eq!(
            titles.get("Text/chapter1.xhtml"),
            Some(&"Chapter 1".to_string())
        );
    }

    #[test]
    fn build_standard_path_preserves_nested_directories() {
        assert_eq!(
            epub_path::standard_path("Text", "Text/part1/chapter.xhtml", &["Text"]),
            "OEBPS/Text/part1/chapter.xhtml"
        );
        assert_eq!(
            epub_path::standard_path("Images", "images/covers/cover.jpg", &["Images", "Image"]),
            "OEBPS/Images/covers/cover.jpg"
        );
    }

    #[test]
    fn allocate_unique_paths_suffixes_collisions_case_insensitively() {
        let mut used = HashSet::new();

        assert_eq!(
            epub_path::allocate_unique_path("OEBPS/Images/cover.jpg", &mut used),
            "OEBPS/Images/cover.jpg"
        );
        assert_eq!(
            epub_path::allocate_unique_path("OEBPS/Images/COVER.jpg", &mut used),
            "OEBPS/Images/COVER-1.jpg"
        );
    }

    #[test]
    fn cover_images_remain_regular_image_resources() {
        let item = ManifestItem {
            id: "cover-image".to_string(),
            href: "Images/cover.jpg".to_string(),
            media_type: "image/jpeg".to_string(),
            properties: Some(vec!["cover".to_string()]),
        };

        assert!(!is_special_file(&item));
    }
}
