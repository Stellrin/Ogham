use super::epub_path;
use super::models::*;
use std::collections::{HashMap, HashSet};

/// 分析 EPUB 结构
pub fn analyze_epub_structure(
    parsed: &ParsedEpub,
    _opf_path: &str,
) -> Result<EpubStructureAnalysis, RefactorError> {
    // 识别章节文件（从 spine）
    let mut chapters = identify_chapters(parsed)?;

    // 分类资源文件
    let mut resources = categorize_resources(parsed, &chapters)?;

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

        // 尝试从 TOC 中获取标题
        let title = extract_title_from_toc(parsed, &manifest_item.href);

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

/// 从 TOC 中提取章节标题
fn extract_title_from_toc(parsed: &ParsedEpub, href: &str) -> Option<String> {
    let toc = parsed.table_of_contents.as_ref()?;

    // 递归查找匹配的章节
    find_title_in_nav_points(&toc.nav_points, href)
}

/// 在导航点中查找标题
fn find_title_in_nav_points(nav_points: &[NavPoint], href: &str) -> Option<String> {
    let normalized_href = epub_path::normalize(href);

    for nav_point in nav_points {
        // 检查当前导航点
        if epub_path::normalize(&nav_point.content_src) == normalized_href {
            return Some(nav_point.label.clone());
        }

        // 递归检查子导航点
        if let Some(title) = find_title_in_nav_points(&nav_point.children, href) {
            return Some(title);
        }
    }

    None
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

        assert_eq!(
            extract_title_from_toc(&parsed, "Text/chapter1.xhtml"),
            Some("Chapter 1".to_string())
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
