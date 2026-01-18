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

    format!("{}-{}", title_part, timestamp)
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

    if let Some(ref nav) = analysis.special_files.nav {
        let filename = nav.rsplit('/').next().unwrap_or(nav);
        manifest_items.push(StandardManifestItem {
            id: "nav".to_string(),
            href: filename.to_string(),
            media_type: "application/xhtml+xml".to_string(),
            properties: Some(vec!["nav".to_string()]),
        });
    }

    if let Some(ref ncx) = analysis.special_files.ncx {
        let filename = ncx.rsplit('/').next().unwrap_or(ncx);
        manifest_items.push(StandardManifestItem {
            id: "ncx".to_string(),
            href: filename.to_string(),
            media_type: "application/x-dtbncx+xml".to_string(),
            properties: None,
        });
    }

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
        // 从现有 TOC 构建导航
        convert_toc_to_navigation(&table_of_contents.nav_points, &analysis.file_map, 0)
    } else {
        // 从章节构建简单导航
        build_simple_navigation(analysis)
    };

    Navigation { toc }
}

/// 转换 TOC 到导航结构
fn convert_toc_to_navigation(
    nav_points: &[NavPoint],
    file_map: &FileMap,
    level: usize,
) -> Vec<TocEntry> {
    nav_points
        .iter()
        .enumerate()
        .map(|(_i, nav_point)| {
            // 获取标准路径，然后转换为相对于 OEBPS/ 的路径
            let content_src = if let Some(standard_path) = file_map.original_to_standard.get(&nav_point.content_src) {
                // 去掉 OEBPS/ 前缀，得到相对路径
                if let Some(rest) = standard_path.strip_prefix("OEBPS/") {
                    rest.to_string()
                } else {
                    standard_path.clone()
                }
            } else {
                nav_point.content_src.clone()
            };

            TocEntry {
                id: nav_point.id.clone(),
                label: nav_point.label.clone(),
                content_src,
                level,
                children: convert_toc_to_navigation(&nav_point.children, file_map, level + 1),
            }
        })
        .collect()
}

/// 构建简单导航（从章节）
fn build_simple_navigation(analysis: &EpubStructureAnalysis) -> Vec<TocEntry> {
    analysis.chapters
        .iter()
        .map(|chapter| {
            // 去掉 OEBPS/ 前缀，得到相对路径
            let content_src = if let Some(rest) = chapter.standard_path.strip_prefix("OEBPS/") {
                rest.to_string()
            } else {
                chapter.standard_path.clone()
            };

            TocEntry {
                id: chapter.id.clone(),
                label: chapter.title.clone().unwrap_or_else(|| {
                    chapter.original_path.rsplit('/').next().unwrap_or(&chapter.original_path).to_string()
                }),
                content_src,
                level: 0,
                children: Vec::new(),
            }
        })
        .collect()
}

/// 将重构结果转换为返回给前端的格式
pub fn convert_to_refactored_result(refactored: &RefactoredEpub) -> RefactoredEpubResult {
    let structure = StandardEpubStructure {
        chapters: refactored.manifest.items
            .iter()
            .filter(|item| {
                item.media_type == "application/xhtml+xml" ||
                item.media_type == "text/html"
            })
            .filter_map(|item| {
                // 从章节中查找标题
                let title = refactored.navigation.toc
                    .iter()
                    .find(|toc| toc.content_src == item.href)
                    .map(|toc| toc.label.clone());

                // 提取文件名
                let original_filename = item.href.rsplit('/').next().unwrap_or(&item.href).to_string();

                // 查找章节顺序
                let order = refactored.spine.itemrefs
                    .iter()
                    .position(|spine_item| spine_item.idref == item.id)
                    .unwrap_or(0);

                Some(StandardChapter {
                    id: item.id.clone(),
                    original_filename,
                    standard_path: item.href.clone(),
                    title,
                    order,
                })
            })
            .collect(),
        styles: refactored.manifest.items
            .iter()
            .filter(|item| item.media_type == "text/css")
            .map(|item| item.href.rsplit('/').next().unwrap_or(&item.href).to_string())
            .collect(),
        images: refactored.manifest.items
            .iter()
            .filter(|item| item.media_type.starts_with("image/"))
            .map(|item| item.href.rsplit('/').next().unwrap_or(&item.href).to_string())
            .collect(),
        fonts: refactored.manifest.items
            .iter()
            .filter(|item| {
                matches!(item.media_type.as_str(),
                    "font/ttf" | "font/otf" | "font/woff" | "font/woff2" |
                    "application/font-ttf" | "application/font-woff" | "application/font-woff2")
            })
            .map(|item| item.href.rsplit('/').next().unwrap_or(&item.href).to_string())
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
