use super::models::*;
use super::parser::parse_epub;
use super::analyzer::analyze_epub_structure;
use super::storage::{get_epub_storage_path, create_standard_directories, save_refactored_epub};
use super::opf_builder::generate_content_opf;
use super::nav_builder::{generate_toc_ncx, generate_nav_xhtml};
use super::resource_manager::{copy_and_organize_files, update_resource_references};

/// 重构 EPUB 文件
pub fn refactor_epub(
    source_path: &str,
) -> Result<RefactoredEpub, RefactorError> {
    // 解析原始 EPUB
    let (parsed, opf_path) = parse_epub(source_path)
        .map_err(|e| RefactorError::ParseError(format!("解析 EPUB 失败: {}", e)))?;

    // 分析 EPUB 结构
    let analysis = analyze_epub_structure(&parsed, &opf_path)?;

    // 生成唯一 ID
    let epub_id = generate_epub_id(&parsed.metadata);

    // 创建标准目录结构
    let storage_path = get_epub_storage_path(&epub_id);
    create_standard_directories(&storage_path)?;

    // 复制和整理文件
    copy_and_organize_files(source_path, &storage_path, &analysis)?;

    // 更新资源引用
    update_resource_references(&storage_path, &analysis)?;

    // 生成 OPF 文件
    let opf_metadata = parsed.metadata.clone();
    let (manifest, spine) = build_standard_structures(&analysis);
    generate_content_opf(&opf_metadata, &manifest, &spine, &storage_path)?;

    // 生成导航文件
    let navigation = build_navigation(&parsed, &analysis);
    generate_toc_ncx(&navigation, &storage_path)?;
    generate_nav_xhtml(&navigation, &storage_path)?;

    // 创建重构后的 EPUB 结构
    let refactored = RefactoredEpub {
        epub_id: epub_id.clone(),
        metadata: parsed.metadata,
        manifest,
        spine,
        navigation,
        storage_path: storage_path.clone(),
        file_map: analysis.file_map,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    // 保存重构后的 EPUB
    save_refactored_epub(&refactored, &epub_id)?;

    Ok(refactored)
}

/// 生成 EPUB ID
fn generate_epub_id(metadata: &EpubMetadata) -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let title_part = metadata.title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(20)
        .collect::<String>();

    let safe_title = if title_part.is_empty() {
        "epub".to_string()
    } else {
        title_part
    };

    format!("{}-{}", safe_title, timestamp)
}

/// 构建标准化清单和阅读顺序
fn build_standard_structures(
    analysis: &EpubStructureAnalysis,
) -> (StandardManifest, StandardSpine) {
    // 构建标准化清单
    let mut manifest_items = Vec::new();

    // 添加章节文件（路径相对于 OEBPS/ 目录）
    for chapter in &analysis.chapters {
        // 将完整路径转换为相对路径（去掉 OEBPS/ 前缀）
        let relative_path = if let Some(rest) = chapter.standard_path.strip_prefix("OEBPS/") {
            rest.to_string()
        } else {
            chapter.standard_path.clone()
        };

        manifest_items.push(StandardManifestItem {
            id: chapter.id.clone(),
            href: relative_path,
            media_type: chapter.media_type.clone(),
            properties: None,
        });
    }

    // 添加样式文件
    for style in &analysis.resources.styles {
        let relative_path = if let Some(rest) = style.standard_path.strip_prefix("OEBPS/") {
            rest.to_string()
        } else {
            style.standard_path.clone()
        };

        manifest_items.push(StandardManifestItem {
            id: style.id.clone(),
            href: relative_path,
            media_type: style.media_type.clone(),
            properties: None,
        });
    }

    // 添加图片文件
    for image in &analysis.resources.images {
        let relative_path = if let Some(rest) = image.standard_path.strip_prefix("OEBPS/") {
            rest.to_string()
        } else {
            image.standard_path.clone()
        };

        manifest_items.push(StandardManifestItem {
            id: image.id.clone(),
            href: relative_path,
            media_type: image.media_type.clone(),
            properties: None,
        });
    }

    // 添加字体文件
    for font in &analysis.resources.fonts {
        let relative_path = if let Some(rest) = font.standard_path.strip_prefix("OEBPS/") {
            rest.to_string()
        } else {
            font.standard_path.clone()
        };

        manifest_items.push(StandardManifestItem {
            id: font.id.clone(),
            href: relative_path,
            media_type: font.media_type.clone(),
            properties: None,
        });
    }

    // 添加特殊文件
    if let Some(ref cover) = analysis.special_files.cover {
        let filename = cover.rsplit('/').next().unwrap_or(cover);
        manifest_items.push(StandardManifestItem {
            id: "cover".to_string(),
            href: format!("Text/{}", filename),
            media_type: "application/xhtml+xml".to_string(),
            properties: Some(vec!["cover".to_string()]),
        });
    }

    // 确保 nav.xhtml 总是在 manifest 中（EPUB 3.0 标准）
    manifest_items.push(StandardManifestItem {
        id: "nav".to_string(),
        href: "nav.xhtml".to_string(),
        media_type: "application/xhtml+xml".to_string(),
        properties: Some(vec!["nav".to_string()]),
    });

    // 确保 toc.ncx 总是在 manifest 中（EPUB 2.0 兼容性）
    manifest_items.push(StandardManifestItem {
        id: "ncx".to_string(),
        href: "toc.ncx".to_string(),
        media_type: "application/x-dtbncx+xml".to_string(),
        properties: None,
    });

    // 构建标准化阅读顺序
    let spine_itemrefs: Vec<StandardSpineItemref> = analysis.chapters
        .iter()
        .map(|chapter| StandardSpineItemref {
            idref: chapter.id.clone(),
            linear: Some(true),
        })
        .collect();

    (
        StandardManifest {
            items: manifest_items,
        },
        StandardSpine {
            itemrefs: spine_itemrefs,
        },
    )
}

/// 构建导航结构
fn build_navigation(
    parsed: &ParsedEpub,
    analysis: &EpubStructureAnalysis,
) -> Navigation {
    let toc = if let Some(ref table_of_contents) = parsed.table_of_contents {
        // 转换现有目录（传入 chapters 参数）
        let converted = convert_toc_to_navigation(
            &table_of_contents.nav_points,
            &analysis.file_map,
            &analysis.chapters,
            0
        );
        // 验证并修复
        validate_and_repair_toc(converted, analysis)
    } else {
        // 无目录时从章节生成
        build_simple_navigation(analysis)
    };

    Navigation { toc }
}

/// 转换 TOC 到导航结构
fn convert_toc_to_navigation(
    nav_points: &[NavPoint],
    file_map: &FileMap,
    chapters: &[ExtendedChapterInfo],
    level: usize,
) -> Vec<TocEntry> {
    nav_points
        .iter()
        .map(|nav_point| {
            // 使用增强的路径解析
            let content_src = resolve_toc_path(&nav_point.content_src, file_map, chapters);

            TocEntry {
                id: nav_point.id.clone(),
                label: nav_point.label.clone(),
                content_src,
                level,
                children: convert_toc_to_navigation(&nav_point.children, file_map, chapters, level + 1),
            }
        })
        .collect()
}

/// 多策略 TOC 路径解析器
fn resolve_toc_path(
    original_path: &str,
    file_map: &FileMap,
    chapters: &[ExtendedChapterInfo],
) -> String {
    // 策略 1: 直接 file_map 查找
    if let Some(standard_path) = file_map.original_to_standard.get(original_path) {
        return strip_oebps_prefix(standard_path);
    }

    // 策略 2: 通过文件名匹配章节
    let filename = extract_filename(original_path);
    if let Some(chapter) = chapters.iter().find(|c| {
        c.original_path.ends_with(&filename)
    }) {
        return strip_oebps_prefix(&chapter.standard_path);
    }

    // 策略 3: 特殊文件处理（nav.xhtml, toc.ncx, cover.xhtml）
    if original_path.contains("nav.xhtml") {
        return "nav.xhtml".to_string();
    }
    if original_path.contains("toc.ncx") {
        return "toc.ncx".to_string();
    }

    // 策略 4: 返回相对路径（将在验证阶段过滤）
    original_path.to_string()
}

/// 去掉 OEBPS/ 前缀（同时处理正斜杠和反斜杠）
fn strip_oebps_prefix(path: &str) -> String {
    path.strip_prefix("OEBPS/")
        .or_else(|| path.strip_prefix("OEBPS\\"))
        .unwrap_or(path)
        .to_string()
}

/// 提取文件名（同时处理正斜杠和反斜杠）
fn extract_filename(path: &str) -> String {
    // 先统一转换为正斜杠，再提取文件名
    let normalized = path.replace('\\', "/");
    normalized.rsplit('/').next().unwrap_or(&normalized).to_string()
}

/// 构建简单导航（从章节）
fn build_simple_navigation(analysis: &EpubStructureAnalysis) -> Vec<TocEntry> {
    analysis.chapters
        .iter()
        .map(|chapter| {
            // 去掉 OEBPS/ 前缀，得到相对路径
            let content_src = strip_oebps_prefix(&chapter.standard_path);

            TocEntry {
                id: chapter.id.clone(),
                label: chapter.title.clone().unwrap_or_else(|| {
                    extract_filename(&chapter.original_path)
                }),
                content_src,
                level: 0,
                children: Vec::new(),
            }
        })
        .collect()
}

/// 验证并修复目录条目
fn validate_and_repair_toc(
    toc_entries: Vec<TocEntry>,
    analysis: &EpubStructureAnalysis,
) -> Vec<TocEntry> {
    // 收集有效章节路径（规范化为正斜杠）
    let valid_chapters: std::collections::HashSet<String> = analysis.chapters
        .iter()
        .map(|c| {
            let path = strip_oebps_prefix(&c.standard_path);
            path.replace('\\', "/")  // 统一使用正斜杠
        })
        .collect();

    toc_entries
        .into_iter()
        .filter_map(|entry| {
            // 规范化路径（统一使用正斜杠）
            let normalized_src = entry.content_src.replace('\\', "/");

            // 检查是否指向有效文件
            let is_valid = valid_chapters.contains(&normalized_src)
                || normalized_src == "nav.xhtml"
                || normalized_src.contains("cover.xhtml");

            if !is_valid {
                println!("[WARN] TOC 条目 '{}' 指向无效文件: {}, 已过滤",
                         entry.label, entry.content_src);
                return None;
            }

            // 递归验证子条目
            let valid_children = validate_and_repair_toc(entry.children, analysis);

            Some(TocEntry {
                children: valid_children,
                ..entry
            })
        })
        .collect()
}

/// 将重构结果转换为返回给前端的格式
pub fn convert_to_refactored_result(refactored: &RefactoredEpub) -> RefactoredEpubResult {
    let structure = StandardEpubStructure {
        chapters: refactored.manifest.items
            .iter()
            .filter(|item| {
                is_renderable_chapter_item(item)
            })
            .filter_map(|item| {
                // 从章节中查找标题
                let title = find_navigation_title(&refactored.navigation.toc, &item.href);

                // 提取文件名
                let original_filename = item.href.rsplit('/').next().unwrap_or(&item.href).to_string();

                // 添加 OEBPS/ 前缀作为标准路径
                let standard_path = format!("OEBPS/{}", item.href);

                // 查找章节顺序
                let order = refactored.spine.itemrefs
                    .iter()
                    .position(|spine_item| spine_item.idref == item.id)
                    .unwrap_or(refactored.spine.itemrefs.len());

                Some(StandardChapter {
                    id: item.id.clone(),
                    original_filename,
                    standard_path,
                    title,
                    order,
                })
            })
            .collect(),
        styles: refactored.manifest.items
            .iter()
            .filter(|item| item.media_type == "text/css")
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        images: refactored.manifest.items
            .iter()
            .filter(|item| item.media_type.starts_with("image/"))
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        fonts: refactored.manifest.items
            .iter()
            .filter(|item| {
                matches!(item.media_type.as_str(),
                    "font/ttf" | "font/otf" | "font/woff" | "font/woff2" |
                    "application/font-ttf" | "application/font-woff" | "application/font-woff2")
            })
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        navigation: convert_navigation_to_frontend(&refactored.navigation.toc),
    };

    RefactoredEpubResult {
        epub_id: refactored.epub_id.clone(),
        metadata: refactored.metadata.clone(),
        structure,
        storage_path: refactored.storage_path.clone(),
    }
}

/// 转换导航结构为前端格式
fn convert_navigation_to_frontend(toc: &[TocEntry]) -> Vec<NavigationEntry> {
    toc.iter().map(|entry| NavigationEntry {
        id: entry.id.clone(),
        label: entry.label.clone(),
        content_src: entry.content_src.clone(),
        level: entry.level,
        children: convert_navigation_to_frontend(&entry.children),
    }).collect()
}

fn is_renderable_chapter_item(item: &StandardManifestItem) -> bool {
    (item.media_type == "application/xhtml+xml" || item.media_type == "text/html")
        && !item
            .properties
            .as_ref()
            .map(|props| props.iter().any(|prop| prop == "nav"))
            .unwrap_or(false)
}

fn find_navigation_title(entries: &[TocEntry], href: &str) -> Option<String> {
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
