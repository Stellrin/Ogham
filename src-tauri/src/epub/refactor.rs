use super::analyzer::analyze_epub_structure;
use super::epub_path;
use super::models::*;
use super::nav_builder::{generate_nav_xhtml, generate_toc_ncx};
use super::opf_builder::generate_content_opf;
use super::parser::parse_epub;
use super::resource_index;
use super::resource_manager::{copy_and_organize_files, update_resource_references};
use super::storage::{create_standard_directories, get_epub_storage_path, save_workspace_metadata};
use std::path::Path;

/// 重构 EPUB 文件
pub fn refactor_epub(source_path: &str) -> Result<RefactoredEpub, RefactorError> {
    // 解析原始 EPUB
    let (parsed, opf_path) = parse_epub(source_path)
        .map_err(|e| RefactorError::ParseError(format!("解析 EPUB 失败: {}", e)))?;

    // 分析 EPUB 结构
    let analysis = analyze_epub_structure(&parsed, &opf_path, source_path)?;

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

    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 创建重构后的 EPUB 结构
    let refactored = RefactoredEpub {
        epub_id: epub_id.clone(),
        metadata: parsed.metadata,
        manifest,
        spine,
        navigation,
        storage_path: storage_path.clone(),
        file_map: analysis.file_map,
        created_at,
    };

    let workspace_metadata = OghamWorkspaceMetadata {
        version: 1,
        epub_id: epub_id.clone(),
        source_path: source_path.to_string(),
        source_name: Path::new(source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Unknown")
            .to_string(),
        file_map: refactored.file_map.clone(),
        created_at,
        updated_at: created_at,
    };
    save_workspace_metadata(&storage_path, &workspace_metadata)?;
    resource_index::refresh_resource_index(&epub_id, &storage_path)?;

    Ok(refactored)
}

/// 生成 EPUB ID
fn generate_epub_id(metadata: &EpubMetadata) -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let title_part = metadata
        .title
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
        let relative_path = epub_path::to_opf_href(&chapter.standard_path);

        manifest_items.push(StandardManifestItem {
            id: chapter.id.clone(),
            href: relative_path,
            media_type: chapter.media_type.clone(),
            properties: None,
        });
    }

    // 添加样式文件
    for style in &analysis.resources.styles {
        let relative_path = epub_path::to_opf_href(&style.standard_path);

        manifest_items.push(StandardManifestItem {
            id: style.id.clone(),
            href: relative_path,
            media_type: style.media_type.clone(),
            properties: None,
        });
    }

    // 添加图片文件
    for image in &analysis.resources.images {
        let relative_path = epub_path::to_opf_href(&image.standard_path);

        manifest_items.push(StandardManifestItem {
            id: image.id.clone(),
            href: relative_path,
            media_type: image.media_type.clone(),
            properties: None,
        });
    }

    // 添加字体文件
    for font in &analysis.resources.fonts {
        let relative_path = epub_path::to_opf_href(&font.standard_path);

        manifest_items.push(StandardManifestItem {
            id: font.id.clone(),
            href: relative_path,
            media_type: font.media_type.clone(),
            properties: None,
        });
    }

    // 添加其他资源文件
    for misc in &analysis.resources.misc {
        let relative_path = epub_path::to_opf_href(&misc.standard_path);

        manifest_items.push(StandardManifestItem {
            id: misc.id.clone(),
            href: relative_path,
            media_type: misc.media_type.clone(),
            properties: None,
        });
    }

    // 添加特殊文件
    if let Some(ref cover) = analysis.special_files.cover {
        let cover_in_spine = analysis
            .chapters
            .iter()
            .any(|chapter| chapter.original_path == *cover);
        if !cover_in_spine {
            if let Some(standard_path) = analysis.file_map.original_to_standard.get(cover) {
                let href = epub_path::strip_oebps_prefix(standard_path);
                if is_text_document_href(&href)
                    && !manifest_items.iter().any(|item| item.href == href)
                {
                    manifest_items.push(StandardManifestItem {
                        id: "cover-page".to_string(),
                        href,
                        media_type: "application/xhtml+xml".to_string(),
                        properties: Some(vec!["cover".to_string()]),
                    });
                }
            }
        }
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
    let spine_itemrefs: Vec<StandardSpineItemref> = analysis
        .chapters
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
fn build_navigation(parsed: &ParsedEpub, analysis: &EpubStructureAnalysis) -> Navigation {
    let toc = if let Some(ref table_of_contents) = parsed.table_of_contents {
        // 转换现有目录（传入 chapters 参数）
        let converted = convert_toc_to_navigation(
            &table_of_contents.nav_points,
            &analysis.file_map,
            &analysis.chapters,
            0,
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
                children: convert_toc_to_navigation(
                    &nav_point.children,
                    file_map,
                    chapters,
                    level + 1,
                ),
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
    let normalized_original = epub_path::normalize_reference(original_path);
    let (lookup_path, suffix) = epub_path::split_reference_suffix(&normalized_original);

    // 策略 1: 直接 file_map 查找
    if let Some(standard_path) = file_map.original_to_standard.get(lookup_path) {
        return format!("{}{}", epub_path::strip_oebps_prefix(standard_path), suffix);
    }

    // 策略 1.1: EPUB 库有时返回 OPF 相对路径，尝试补 OEBPS 前缀
    let lookup_with_oebps = format!("OEBPS/{}", lookup_path.trim_start_matches('/'));
    if let Some(standard_path) = file_map.original_to_standard.get(&lookup_with_oebps) {
        return format!("{}{}", epub_path::strip_oebps_prefix(standard_path), suffix);
    }

    // 策略 2: 通过文件名匹配章节
    let filename = epub_path::filename(lookup_path);
    if let Some(chapter) = chapters
        .iter()
        .find(|c| c.original_path.replace('\\', "/").ends_with(&filename))
    {
        return format!(
            "{}{}",
            epub_path::strip_oebps_prefix(&chapter.standard_path),
            suffix
        );
    }

    // 策略 3: 特殊文件处理（nav.xhtml, toc.ncx, cover.xhtml）
    if lookup_path.contains("nav.xhtml") {
        return format!("nav.xhtml{}", suffix);
    }
    if lookup_path.contains("toc.ncx") {
        return format!("toc.ncx{}", suffix);
    }

    // 策略 4: 返回相对路径（将在验证阶段过滤）
    normalized_original
}

/// 构建简单导航（从章节）
fn build_simple_navigation(analysis: &EpubStructureAnalysis) -> Vec<TocEntry> {
    analysis
        .chapters
        .iter()
        .map(|chapter| {
            // 去掉 OEBPS/ 前缀，得到相对路径
            let content_src = epub_path::strip_oebps_prefix(&chapter.standard_path);

            TocEntry {
                id: chapter.id.clone(),
                label: chapter
                    .title
                    .clone()
                    .unwrap_or_else(|| epub_path::filename(&chapter.original_path)),
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
    let valid_chapters: std::collections::HashSet<String> = analysis
        .chapters
        .iter()
        .map(|c| {
            let path = epub_path::strip_oebps_prefix(&c.standard_path);
            path.replace('\\', "/") // 统一使用正斜杠
        })
        .collect();

    toc_entries
        .into_iter()
        .filter_map(|entry| {
            // 规范化路径（统一使用正斜杠）
            let normalized_src = entry.content_src.replace('\\', "/");
            let normalized_src_base = epub_path::split_reference_suffix(&normalized_src).0;

            // 检查是否指向有效文件
            let is_valid = valid_chapters.contains(normalized_src_base)
                || normalized_src_base == "nav.xhtml"
                || normalized_src_base.contains("cover.xhtml");

            if !is_valid {
                println!(
                    "[WARN] TOC 条目 '{}' 指向无效文件: {}, 已过滤",
                    entry.label, entry.content_src
                );
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
        chapters: refactored
            .manifest
            .items
            .iter()
            .filter(|item| is_renderable_chapter_item(item))
            .filter_map(|item| {
                // 从章节中查找标题
                let title = find_navigation_title(&refactored.navigation.toc, &item.href);

                // 提取文件名
                let original_filename = item
                    .href
                    .rsplit('/')
                    .next()
                    .unwrap_or(&item.href)
                    .to_string();

                // 添加 OEBPS/ 前缀作为标准路径
                let standard_path = format!("OEBPS/{}", item.href);

                // 查找章节顺序
                let order = refactored
                    .spine
                    .itemrefs
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
        styles: refactored
            .manifest
            .items
            .iter()
            .filter(|item| item.media_type == "text/css")
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        images: refactored
            .manifest
            .items
            .iter()
            .filter(|item| item.media_type.starts_with("image/"))
            .map(|item| format!("OEBPS/{}", item.href))
            .collect(),
        fonts: refactored
            .manifest
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
    toc.iter()
        .map(|entry| NavigationEntry {
            id: entry.id.clone(),
            label: entry.label.clone(),
            content_src: entry.content_src.clone(),
            level: entry.level,
            children: convert_navigation_to_frontend(&entry.children),
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

fn is_text_document_href(href: &str) -> bool {
    let lower = href.to_lowercase();
    lower.ends_with(".xhtml") || lower.ends_with(".html")
}

fn find_navigation_title(entries: &[TocEntry], href: &str) -> Option<String> {
    let normalized_href = href.split('#').next().unwrap_or(href);

    for entry in entries {
        if entry
            .content_src
            .split('#')
            .next()
            .unwrap_or(&entry.content_src)
            == normalized_href
        {
            return Some(entry.label.clone());
        }

        if let Some(title) = find_navigation_title(&entry.children, href) {
            return Some(title);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::Write;
    use zip::{write::FileOptions, ZipWriter};

    #[test]
    fn resolve_toc_path_preserves_anchor() {
        let mut original_to_standard = HashMap::new();
        original_to_standard.insert(
            "OEBPS/Text/chapter1.xhtml".to_string(),
            "OEBPS/Text/chapter1.xhtml".to_string(),
        );

        let file_map = FileMap {
            original_to_standard,
            standard_to_original: HashMap::new(),
        };
        let chapters = vec![ExtendedChapterInfo {
            id: "chapter1".to_string(),
            original_path: "OEBPS/Text/chapter1.xhtml".to_string(),
            standard_path: "OEBPS/Text/chapter1.xhtml".to_string(),
            title: Some("Chapter 1".to_string()),
            order: 0,
            media_type: "application/xhtml+xml".to_string(),
        }];

        let resolved = resolve_toc_path("Text/chapter1.xhtml#part-2", &file_map, &chapters);
        assert_eq!(resolved, "Text/chapter1.xhtml#part-2");
    }

    #[test]
    fn validate_toc_accepts_valid_anchor_targets() {
        let analysis = EpubStructureAnalysis {
            chapters: vec![ExtendedChapterInfo {
                id: "chapter1".to_string(),
                original_path: "OEBPS/Text/chapter1.xhtml".to_string(),
                standard_path: "OEBPS/Text/chapter1.xhtml".to_string(),
                title: Some("Chapter 1".to_string()),
                order: 0,
                media_type: "application/xhtml+xml".to_string(),
            }],
            resources: ResourceCollection {
                styles: Vec::new(),
                images: Vec::new(),
                fonts: Vec::new(),
                misc: Vec::new(),
            },
            special_files: SpecialFiles {
                cover: None,
                nav: None,
                ncx: None,
            },
            file_map: FileMap {
                original_to_standard: HashMap::new(),
                standard_to_original: HashMap::new(),
            },
        };

        let entries = vec![TocEntry {
            id: "nav-1".to_string(),
            label: "Chapter 1".to_string(),
            content_src: "Text/chapter1.xhtml#part-2".to_string(),
            level: 0,
            children: Vec::new(),
        }];

        let repaired = validate_and_repair_toc(entries, &analysis);
        assert_eq!(repaired.len(), 1);
        assert_eq!(repaired[0].content_src, "Text/chapter1.xhtml#part-2");
    }

    #[test]
    fn refactor_copies_images_referenced_outside_manifest() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture_path = std::env::temp_dir().join(format!(
            "ogham-unmanifested-image-{}.epub",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        create_unmanifested_image_epub(&fixture_path)?;

        let fixture_path_str = fixture_path.to_string_lossy().to_string();
        let refactored = refactor_epub(&fixture_path_str)?;
        let storage_path = std::path::Path::new(&refactored.storage_path);
        let copied_image = storage_path.join("OEBPS/Images/pic.jpg");
        let rewritten_chapter = fs::read_to_string(storage_path.join("OEBPS/Text/chapter.xhtml"))?;

        assert!(copied_image.exists());
        assert!(refactored
            .manifest
            .items
            .iter()
            .any(|item| item.href == "Images/pic.jpg" && item.media_type == "image/jpeg"));
        assert!(rewritten_chapter.contains("src=\"../Images/pic.jpg\""));

        let refactored_zip = std::env::temp_dir()
            .join("ogham-library")
            .join(format!("{}_refactored.epub", refactored.epub_id));
        let _ = fs::remove_file(refactored_zip);
        let _ = fs::remove_dir_all(&refactored.storage_path);
        let _ = fs::remove_file(fixture_path);

        Ok(())
    }

    #[test]
    fn refactor_decodes_percent_encoded_manifest_hrefs() -> Result<(), Box<dyn std::error::Error>> {
        let fixture_path = std::env::temp_dir().join(format!(
            "ogham-percent-encoded-{}.epub",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        create_percent_encoded_epub(&fixture_path)?;

        let fixture_path_str = fixture_path.to_string_lossy().to_string();
        let refactored = refactor_epub(&fixture_path_str)?;
        let copied_chapter = std::path::Path::new(&refactored.storage_path)
            .join("OEBPS/Text/Character Profile1.xhtml");

        assert!(copied_chapter.exists());
        assert!(refactored
            .manifest
            .items
            .iter()
            .any(|item| item.href == "Text/Character Profile1.xhtml"));

        let refactored_zip = std::env::temp_dir()
            .join("ogham-library")
            .join(format!("{}_refactored.epub", refactored.epub_id));
        let _ = fs::remove_file(refactored_zip);
        let _ = fs::remove_dir_all(&refactored.storage_path);
        let _ = fs::remove_file(fixture_path);

        Ok(())
    }

    fn create_percent_encoded_epub(
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        zip.start_file(
            "mimetype",
            FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored),
        )?;
        zip.write_all(b"application/epub+zip")?;

        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;

        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Percent Encoded Fixture</dc:title>
    <dc:identifier id="BookId">percent-encoded-fixture</dc:identifier>
  </metadata>
  <manifest>
    <item href="toc.ncx" id="ncx" media-type="application/x-dtbncx+xml" />
    <item href="Text/Character%20Profile1.xhtml" id="chapter" media-type="application/xhtml+xml" />
  </manifest>
  <spine toc="ncx">
    <itemref idref="chapter" />
  </spine>
</package>"#,
        )?;

        zip.start_file(
            "OEBPS/Text/Character Profile1.xhtml",
            FileOptions::<()>::default(),
        )?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Character Profile</title></head>
  <body><p>ok</p></body>
</html>"#,
        )?;

        zip.start_file("OEBPS/toc.ncx", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta content="percent-encoded-fixture" name="dtb:uid"/>
  </head>
  <docTitle><text>Percent Encoded Fixture</text></docTitle>
  <navMap>
    <navPoint id="navPoint-1" playOrder="1">
      <navLabel><text>Character Profile</text></navLabel>
      <content src="Text/Character%20Profile1.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"#,
        )?;

        zip.finish()?;
        Ok(())
    }

    fn create_unmanifested_image_epub(
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        zip.start_file(
            "mimetype",
            FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored),
        )?;
        zip.write_all(b"application/epub+zip")?;

        zip.start_file("META-INF/container.xml", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;

        zip.start_file("OEBPS/content.opf", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Unmanifested Image Fixture</dc:title>
    <dc:identifier id="BookId">unmanifested-image-fixture</dc:identifier>
  </metadata>
  <manifest>
    <item href="toc.ncx" id="ncx" media-type="application/x-dtbncx+xml" />
    <item href="Text/chapter.xhtml" id="chapter" media-type="application/xhtml+xml" />
  </manifest>
  <spine toc="ncx">
    <itemref idref="chapter" />
  </spine>
</package>"#,
        )?;

        zip.start_file("OEBPS/Text/chapter.xhtml", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter</title></head>
  <body><p><img src="../images/pic.jpg" alt="pic" /></p></body>
</html>"#,
        )?;

        zip.start_file("OEBPS/images/pic.jpg", FileOptions::<()>::default())?;
        zip.write_all(b"fake-jpeg")?;

        zip.start_file("OEBPS/toc.ncx", FileOptions::<()>::default())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta content="unmanifested-image-fixture" name="dtb:uid"/>
  </head>
  <docTitle><text>Unmanifested Image Fixture</text></docTitle>
  <navMap>
    <navPoint id="navPoint-1" playOrder="1">
      <navLabel><text>Chapter</text></navLabel>
      <content src="Text/chapter.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"#,
        )?;

        zip.finish()?;
        Ok(())
    }
}
