use crate::epub::epub_path;
use crate::epub::models::{
    Navigation, RefactorError, StandardManifest, StandardSpine, StandardSpineItemref,
    TocChapterDto, TocEntry, TocOrderDto,
};
use crate::epub::opf_document;
use crate::epub::resource_index;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 目录管理器 - 负责解析、更新、同步目录与 OPF/导航文件
pub struct TocManager;

impl TocManager {
    /// 从标准化后的管理目录加载并解析目录到文件的映射
    pub fn load_toc_from_storage(storage_path: &str) -> Result<Vec<TocChapterDto>, RefactorError> {
        let nav_path = Path::new(storage_path).join("OEBPS/nav.xhtml");
        let ncx_path = Path::new(storage_path).join("OEBPS/toc.ncx");

        let entries = if nav_path.exists() {
            // 优先尝试加载 nav.xhtml
            Self::load_from_nav(&nav_path.to_string_lossy(), storage_path)?
        } else if ncx_path.exists() {
            // 回退到 toc.ncx
            Self::load_from_ncx(&ncx_path.to_string_lossy(), storage_path)?
        } else {
            return Err(RefactorError::MissingFile(
                "No navigation file found (nav.xhtml or toc.ncx)".to_string(),
            ));
        };

        let entries = Self::prune_missing_toc_targets(entries, storage_path);
        let entries = Self::normalize_toc_entries(entries);
        Self::attach_spine_file_groups(entries, storage_path)
    }

    /// 从 nav.xhtml 加载目录
    fn load_from_nav(nav_path: &str, base_path: &str) -> Result<Vec<TocChapterDto>, RefactorError> {
        let content = fs::read_to_string(nav_path)
            .map_err(|e| RefactorError::IoError(format!("Failed to read nav.xhtml: {}", e)))?;

        // 检测格式：EPUB 3.0 使用 <nav epub:type="toc">，EPUB 2.0 使用 <navMap>
        if content.contains("<navMap") {
            // EPUB 2.0 NCX 格式
            Self::load_from_nav_ncx_format(&content, base_path)
        } else {
            // EPUB 3.0 HTML 格式
            Self::load_from_nav_epub3(&content, base_path)
        }
    }

    /// 从 EPUB 3.0 nav.xhtml 加载目录（HTML 格式）
    fn load_from_nav_epub3(
        content: &str,
        base_path: &str,
    ) -> Result<Vec<TocChapterDto>, RefactorError> {
        let mut entries = Vec::new();
        let mut order_counter = 0usize;

        if let Some(nav_content) = Self::find_toc_nav_content(content) {
            let ol_content = nav_content
                .find("<ol")
                .and_then(|ol_start| {
                    Self::find_matching_close_html(&nav_content[ol_start..], "<ol", "</ol>")
                        .map(|ol_end| &nav_content[ol_start..ol_start + ol_end])
                })
                .unwrap_or(nav_content);

            Self::parse_ol_elements(ol_content, 0, &mut entries, &mut order_counter, base_path);
        }

        Ok(entries)
    }

    fn find_toc_nav_content(content: &str) -> Option<&str> {
        let mut search_pos = 0;
        let mut first_list_nav = None;

        while let Some(nav_start) = content[search_pos..].find("<nav") {
            let nav_start = search_pos + nav_start;
            let nav_end =
                match Self::find_matching_close_html(&content[nav_start..], "<nav", "</nav>") {
                    Some(end) => nav_start + end,
                    None => {
                        search_pos = nav_start + 4;
                        continue;
                    }
                };

            let nav_content = &content[nav_start..nav_end];
            if Self::is_toc_nav(nav_content) {
                return Some(nav_content);
            }

            if first_list_nav.is_none() && nav_content.contains("<ol") {
                first_list_nav = Some(nav_content);
            }

            search_pos = nav_end;
        }

        first_list_nav
    }

    fn is_toc_nav(nav_content: &str) -> bool {
        let start_tag_end = nav_content.find('>').unwrap_or(nav_content.len());
        let start_tag = &nav_content[..start_tag_end];

        start_tag.contains("epub:type=\"toc\"")
            || start_tag.contains("epub:type='toc'")
            || start_tag.contains("type=\"toc\"")
            || start_tag.contains("type='toc'")
    }

    /// 解析 <ol> 元素中的所有 <li> 项
    fn parse_ol_elements(
        content: &str,
        level: usize,
        entries: &mut Vec<TocChapterDto>,
        order_counter: &mut usize,
        base_path: &str,
    ) {
        let mut search_pos = 0;
        while let Some(li_start) = content[search_pos..].find("<li") {
            let li_start = search_pos + li_start;
            // 找到 </li> 结束位置
            let li_end = match Self::find_matching_close_html(&content[li_start..], "<li", "</li>")
            {
                Some(end) => li_start + end,
                None => {
                    search_pos = li_start + 3;
                    continue;
                }
            };

            let li_content = &content[li_start..li_end];

            // 提取 <a> 标签中的文本和链接
            if let Some(a_start) = li_content.find("<a ") {
                if let Some(a_end) = li_content[a_start..].find("</a>") {
                    let a_content = &li_content[a_start..a_start + a_end + 4];

                    // 提取 href
                    let href = Self::extract_html_attribute(a_content, "href").unwrap_or_default();

                    // 提取文本内容
                    let label = Self::extract_text_content(a_content)
                        .unwrap_or_else(|| "Untitled".to_string());

                    if !href.is_empty() || !label.is_empty() {
                        let order = *order_counter;
                        *order_counter += 1;

                        // 解析文件路径
                        let file_path = Self::parse_content_src(&href, base_path);

                        // 检查是否有子 <ol>
                        let mut children = Vec::new();
                        if let Some(ol_start) = li_content.find("<ol") {
                            let ol_end = Self::find_matching_close_html(
                                &li_content[ol_start..],
                                "<ol",
                                "</ol>",
                            )
                            .map(|end| ol_start + end)
                            .unwrap_or(li_content.len());
                            let ol_content = &li_content[ol_start..ol_end];
                            Self::parse_ol_elements(
                                ol_content,
                                level + 1,
                                &mut children,
                                order_counter,
                                base_path,
                            );
                        }

                        entries.push(TocChapterDto {
                            id: format!("nav-{}", order),
                            label,
                            content_src: href,
                            file_path: Some(file_path),
                            file_paths: Vec::new(),
                            file_name: None,
                            file_names: Vec::new(),
                            anchor: None,
                            level,
                            order,
                            children,
                        });
                    }
                }
            }

            search_pos = li_end;
        }
    }

    /// 从 EPUB 2.0 NCX 格式加载（复用原有逻辑）
    fn load_from_nav_ncx_format(
        content: &str,
        base_path: &str,
    ) -> Result<Vec<TocChapterDto>, RefactorError> {
        let mut entries = Vec::new();
        let mut order_counter = 0usize;

        Self::parse_nav_elements(content, 0, &mut entries, &mut order_counter, base_path);

        Ok(entries)
    }

    /// 提取 HTML 属性值
    fn extract_html_attribute(content: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = content.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = content[value_start..].find('"') {
                return Some(content[value_start..value_start + end].to_string());
            }
        }
        None
    }

    /// 提取纯文本内容（去除 HTML 标签）
    fn extract_text_content(content: &str) -> Option<String> {
        // 找到 > 之后的内容
        if let Some(start) = content.find('>') {
            let text = &content[start + 1..];
            // 去掉 </a> 之前的内容
            if let Some(end) = text.find("</a>") {
                let text = &text[..end];
                // 去除空白并返回
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    /// 查找匹配的 HTML 闭合标签（处理自闭合标签）
    fn find_matching_close_html(content: &str, open_tag: &str, close_tag: &str) -> Option<usize> {
        let mut depth = 0;
        let mut search_pos = 0;
        let open_tag_lower = open_tag.to_lowercase();
        let close_tag_lower = close_tag.to_lowercase();

        while search_pos < content.len() {
            // 确保 search_pos 在字符边界上，避免 UTF-8 多字节字符分割问题
            if !content.is_char_boundary(search_pos) {
                search_pos += 1;
                continue;
            }
            // 检查是否是自闭合标签（如 <br/>, <img/> 等）
            if starts_with_ignore_ascii_case_at(content, search_pos, &open_tag_lower) {
                // 检查是否有 /> 自闭合
                let after_open = &content[search_pos + open_tag.len()..];
                if after_open.starts_with("/>")
                    || after_open.starts_with(" ") && after_open.contains("/>")
                {
                    // 自闭合标签，不增加深度
                    search_pos += open_tag.len() + 2;
                    continue;
                }
                depth += 1;
                search_pos += open_tag.len();
            } else if starts_with_ignore_ascii_case_at(content, search_pos, &close_tag_lower) {
                depth -= 1;
                if depth == 0 {
                    return Some(search_pos + close_tag.len());
                }
                search_pos += close_tag.len();
            } else {
                search_pos += 1;
            }

            if depth == 0 && close_tag.len() > 0 {
                break;
            }
        }

        None
    }

    fn parse_navpoint_elements(
        content: &str,
        level: usize,
        entries: &mut Vec<TocChapterDto>,
        order_counter: &mut usize,
        base_path: &str,
    ) {
        let mut search_pos = 0;
        while let Some(point_start) = content[search_pos..].find("<navPoint") {
            let point_start = search_pos + point_start;
            let point_end =
                Self::find_matching_close(&content[point_start..], "<navPoint", "</navPoint>")
                    .map(|end| point_start + end)
                    .unwrap_or(content.len());

            let nav_point_content = &content[point_start..point_end];

            // 提取 id
            let id = Self::extract_attribute(nav_point_content, "id")
                .unwrap_or_else(|| format!("navpoint-{}", *order_counter));

            // 提取 label (从 text 元素)
            let label = Self::extract_nav_label(nav_point_content)
                .unwrap_or_else(|| "Untitled".to_string());

            // 提取 content src
            let content_src = Self::extract_attribute(nav_point_content, "src")
                .or_else(|| Self::extract_content_src(nav_point_content))
                .unwrap_or_default();

            // 解析文件路径
            let file_path = Self::parse_content_src(&content_src, base_path);

            // 递归处理子元素
            let mut children = Vec::new();
            let child_content_start = nav_point_content["<navPoint".len()..]
                .find("<navPoint")
                .map(|pos| pos + "<navPoint".len());
            if let Some(child_start) = child_content_start {
                Self::parse_navpoint_elements(
                    &nav_point_content[child_start..],
                    level + 1,
                    &mut children,
                    order_counter,
                    base_path,
                );
            }

            let order = *order_counter;
            *order_counter += 1;

            entries.push(TocChapterDto {
                id,
                label,
                content_src,
                file_path: Some(file_path),
                file_paths: Vec::new(),
                file_name: None,
                file_names: Vec::new(),
                anchor: None,
                level,
                order,
                children,
            });

            search_pos = point_end;
        }
    }

    /// 递归解析 nav 元素
    fn parse_nav_elements(
        content: &str,
        level: usize,
        entries: &mut Vec<TocChapterDto>,
        order_counter: &mut usize,
        base_path: &str,
    ) {
        // 使用简单的字符串匹配解析 navMap 中的 navPoint
        let nav_map_start = match content.find("<navMap") {
            Some(pos) => pos,
            None => return,
        };

        let nav_map_content = &content[nav_map_start..];
        Self::parse_navpoint_elements(nav_map_content, level, entries, order_counter, base_path);
    }

    /// 从 toc.ncx 加载目录
    fn load_from_ncx(ncx_path: &str, base_path: &str) -> Result<Vec<TocChapterDto>, RefactorError> {
        let content = fs::read_to_string(ncx_path)
            .map_err(|e| RefactorError::IoError(format!("Failed to read toc.ncx: {}", e)))?;

        let mut entries = Vec::new();
        let mut order_counter = 0usize;

        // 解析 navMap
        let nav_map_start = match content.find("<navMap") {
            Some(pos) => pos,
            None => return Ok(entries),
        };

        let nav_map_content = &content[nav_map_start..];
        Self::parse_ncx_navmap(
            nav_map_content,
            0,
            &mut entries,
            &mut order_counter,
            base_path,
        );

        Ok(entries)
    }

    /// 解析 NCX navMap
    fn parse_ncx_navmap(
        content: &str,
        level: usize,
        entries: &mut Vec<TocChapterDto>,
        order_counter: &mut usize,
        base_path: &str,
    ) {
        let mut search_pos = 0;
        while let Some(point_start) = content[search_pos..].find("<navPoint") {
            let point_start = search_pos + point_start;
            let point_end =
                Self::find_matching_close(&content[point_start..], "<navPoint", "</navPoint>")
                    .map(|end| point_start + end)
                    .unwrap_or(content.len());

            let nav_point_content = &content[point_start..point_end];

            let id = Self::extract_attribute(nav_point_content, "id")
                .unwrap_or_else(|| format!("navpoint-{}", *order_counter));

            let label = Self::extract_ncx_label(nav_point_content)
                .unwrap_or_else(|| "Untitled".to_string());

            let content_src = Self::extract_ncx_content_src(nav_point_content).unwrap_or_default();

            let file_path = Self::parse_content_src(&content_src, base_path);

            let mut children = Vec::new();
            let child_content_start = nav_point_content["<navPoint".len()..]
                .find("<navPoint")
                .map(|pos| pos + "<navPoint".len());
            if let Some(child_start) = child_content_start {
                Self::parse_ncx_navmap(
                    &nav_point_content[child_start..],
                    level + 1,
                    &mut children,
                    order_counter,
                    base_path,
                );
            }

            let order = *order_counter;
            *order_counter += 1;

            entries.push(TocChapterDto {
                id,
                label,
                content_src,
                file_path: Some(file_path),
                file_paths: Vec::new(),
                file_name: None,
                file_names: Vec::new(),
                anchor: None,
                level,
                order,
                children,
            });

            search_pos = point_end;
        }
    }

    /// 从 content_src 提取实际文件路径
    /// 返回形如 "OEBPS/Text/xxx.xhtml" 的路径，与 StandardChapter.standard_path 保持一致
    pub fn parse_content_src(content_src: &str, _base_path: &str) -> String {
        epub_path::to_standard_content_path(content_src)
    }

    fn normalize_toc_entries(entries: Vec<TocChapterDto>) -> Vec<TocChapterDto> {
        let deduped = Self::dedupe_toc_entries(entries);
        let mut order_counter = 0usize;
        Self::renumber_toc_entries(deduped, 0, &mut order_counter)
    }

    fn attach_spine_file_groups(
        entries: Vec<TocChapterDto>,
        storage_path: &str,
    ) -> Result<Vec<TocChapterDto>, RefactorError> {
        let document = opf_document::load_opf_document(storage_path, "unknown")?;
        let ordered_chapters = Self::ordered_chapter_paths(&document.manifest, &document.spine);
        Ok(Self::attach_spine_file_groups_to_entries(
            entries,
            &ordered_chapters,
        ))
    }

    fn ordered_chapter_paths(manifest: &StandardManifest, spine: &StandardSpine) -> Vec<String> {
        let manifest_by_id: HashMap<&str, &crate::epub::models::StandardManifestItem> = manifest
            .items
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();

        spine
            .itemrefs
            .iter()
            .filter_map(|itemref| {
                manifest_by_id
                    .get(itemref.idref.as_str())
                    .copied()
                    .filter(|item| Self::is_renderable_chapter_item(item))
                    .map(|item| format!("OEBPS/{}", item.href))
            })
            .collect()
    }

    fn is_renderable_chapter_item(item: &crate::epub::models::StandardManifestItem) -> bool {
        (item.media_type == "application/xhtml+xml" || item.media_type == "text/html")
            && item.href.replace('\\', "/").starts_with("Text/")
            && !item
                .properties
                .as_ref()
                .map(|props| props.iter().any(|prop| prop == "nav"))
                .unwrap_or(false)
    }

    fn attach_spine_file_groups_to_entries(
        mut entries: Vec<TocChapterDto>,
        ordered_chapters: &[String],
    ) -> Vec<TocChapterDto> {
        let chapter_index_by_path = Self::chapter_index_by_path(ordered_chapters);
        let mut target_indices = Vec::new();
        Self::collect_toc_target_indices(&entries, &chapter_index_by_path, &mut target_indices);
        target_indices.sort_unstable();
        target_indices.dedup();

        Self::attach_spine_file_groups_recursive(
            &mut entries,
            ordered_chapters,
            &chapter_index_by_path,
            &target_indices,
        );
        entries
    }

    fn chapter_index_by_path(ordered_chapters: &[String]) -> HashMap<String, usize> {
        ordered_chapters
            .iter()
            .enumerate()
            .map(|(index, chapter_path)| (epub_path::casefold(chapter_path), index))
            .collect()
    }

    fn collect_toc_target_indices(
        entries: &[TocChapterDto],
        chapter_index_by_path: &HashMap<String, usize>,
        target_indices: &mut Vec<usize>,
    ) {
        for entry in entries {
            if let Some(index) = Self::chapter_index_for_entry(entry, chapter_index_by_path) {
                target_indices.push(index);
            }

            Self::collect_toc_target_indices(
                &entry.children,
                chapter_index_by_path,
                target_indices,
            );
        }
    }

    fn attach_spine_file_groups_recursive(
        entries: &mut [TocChapterDto],
        ordered_chapters: &[String],
        chapter_index_by_path: &HashMap<String, usize>,
        target_indices: &[usize],
    ) {
        for entry in entries {
            Self::attach_spine_file_groups_recursive(
                &mut entry.children,
                ordered_chapters,
                chapter_index_by_path,
                target_indices,
            );
            Self::attach_file_metadata(
                entry,
                ordered_chapters,
                chapter_index_by_path,
                target_indices,
            );
        }
    }

    fn attach_file_metadata(
        entry: &mut TocChapterDto,
        ordered_chapters: &[String],
        chapter_index_by_path: &HashMap<String, usize>,
        target_indices: &[usize],
    ) {
        entry.anchor = Self::entry_anchor(entry);

        let Some(target_index) = Self::chapter_index_for_entry(entry, chapter_index_by_path) else {
            Self::fill_file_display_fields(entry);
            return;
        };

        let next_target_position =
            target_indices.partition_point(|candidate| *candidate <= target_index);
        let next_target_index = target_indices
            .get(next_target_position)
            .copied()
            .unwrap_or(ordered_chapters.len());

        let file_paths: Vec<String> = ordered_chapters[target_index..next_target_index].to_vec();
        if let Some(primary_path) = file_paths.first() {
            entry.file_path = Some(primary_path.clone());
        }
        entry.file_paths = file_paths;
        Self::fill_file_display_fields(entry);
    }

    fn chapter_index_for_entry(
        entry: &TocChapterDto,
        chapter_index_by_path: &HashMap<String, usize>,
    ) -> Option<usize> {
        let target_path = entry.file_path.as_deref()?;
        let target_key = epub_path::casefold(target_path);
        chapter_index_by_path.get(&target_key).copied()
    }

    fn fill_file_display_fields(entry: &mut TocChapterDto) {
        if entry.file_paths.is_empty() {
            if let Some(file_path) = entry.file_path.clone() {
                entry.file_paths.push(file_path);
            }
        }

        entry.file_name = entry
            .file_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .map(Self::file_name);
        entry.file_names = entry
            .file_paths
            .iter()
            .map(|path| Self::file_name(path))
            .collect();
    }

    fn file_name(path: &str) -> String {
        epub_path::filename(path)
    }

    fn entry_anchor(entry: &TocChapterDto) -> Option<String> {
        epub_path::reference_anchor(&entry.content_src)
    }

    fn prune_missing_toc_targets(
        entries: Vec<TocChapterDto>,
        storage_path: &str,
    ) -> Vec<TocChapterDto> {
        entries
            .into_iter()
            .filter_map(|mut entry| {
                entry.children = Self::prune_missing_toc_targets(entry.children, storage_path);

                if Self::toc_target_exists(&entry, storage_path) || !entry.children.is_empty() {
                    Some(entry)
                } else {
                    None
                }
            })
            .collect()
    }

    fn toc_target_exists(entry: &TocChapterDto, storage_path: &str) -> bool {
        let content_src = entry.content_src.trim();
        if content_src.is_empty() || content_src.starts_with('#') {
            return true;
        }

        let Some(file_path) = entry
            .file_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
        else {
            return false;
        };

        Path::new(storage_path).join(file_path).exists()
    }

    fn dedupe_toc_entries(entries: Vec<TocChapterDto>) -> Vec<TocChapterDto> {
        let mut seen = std::collections::HashSet::new();
        let mut deduped_reversed = Vec::new();

        for mut entry in entries.into_iter().rev() {
            entry.children = Self::dedupe_toc_entries(entry.children);
            let key = Self::toc_entry_identity_key(&entry);
            if seen.insert(key) {
                deduped_reversed.push(entry);
            }
        }

        deduped_reversed.reverse();
        deduped_reversed
    }

    fn renumber_toc_entries(
        entries: Vec<TocChapterDto>,
        level: usize,
        order_counter: &mut usize,
    ) -> Vec<TocChapterDto> {
        entries
            .into_iter()
            .map(|mut entry| {
                let order = *order_counter;
                *order_counter += 1;
                entry.level = level;
                entry.order = order;
                entry.children =
                    Self::renumber_toc_entries(entry.children, level + 1, order_counter);
                entry
            })
            .collect()
    }

    fn toc_entry_identity_key(entry: &TocChapterDto) -> String {
        let label = entry
            .label
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let target = Self::canonical_toc_target(entry);
        format!("{}|{}", label, target)
    }

    fn canonical_toc_target(entry: &TocChapterDto) -> String {
        let source = entry
            .file_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .unwrap_or(&entry.content_src);
        let suffix = epub_path::split_reference_suffix(&entry.content_src).1;
        let standard_path = Self::parse_content_src(source, "");

        format!("{}{}", epub_path::casefold(&standard_path), suffix)
    }

    /// 更新导航文件 (nav.xhtml)
    pub fn update_nav_file(toc: &[TocChapterDto], base_path: &str) -> Result<(), RefactorError> {
        let nav_path = Path::new(base_path).join("OEBPS/nav.xhtml");

        if !nav_path.exists() {
            return Err(RefactorError::MissingFile(
                "nav.xhtml not found".to_string(),
            ));
        }

        let content = fs::read_to_string(&nav_path)
            .map_err(|e| RefactorError::IoError(format!("Failed to read nav.xhtml: {}", e)))?;

        let navigation = Self::toc_to_navigation(toc);
        let new_content = if content.contains("<nav") {
            crate::epub::nav_builder::render_nav_xhtml(&navigation)?
        } else {
            content
        };

        fs::write(&nav_path, new_content)
            .map_err(|e| RefactorError::IoError(format!("Failed to write nav.xhtml: {}", e)))?;

        Ok(())
    }

    /// 更新 NCX 文件
    pub fn update_ncx_file(toc: &[TocChapterDto], base_path: &str) -> Result<(), RefactorError> {
        let ncx_path = Path::new(base_path).join("OEBPS/toc.ncx");

        if !ncx_path.exists() {
            return Ok(()); // NCX 不存在则跳过
        }

        let content = fs::read_to_string(&ncx_path)
            .map_err(|e| RefactorError::IoError(format!("Failed to read toc.ncx: {}", e)))?;

        let navigation = Self::toc_to_navigation(toc);
        let new_content = if content.contains("<navMap") {
            crate::epub::nav_builder::render_toc_ncx(&navigation)?
        } else {
            content
        };

        fs::write(&ncx_path, new_content)
            .map_err(|e| RefactorError::IoError(format!("Failed to write toc.ncx: {}", e)))?;

        Ok(())
    }

    /// 更新 OPF spine 顺序
    pub fn update_spine_order(
        toc: &[TocChapterDto],
        manifest: &StandardManifest,
        base_path: &str,
    ) -> Result<(), RefactorError> {
        // 从目录展平获取所有内容文件
        let flat_toc: Vec<TocChapterDto> = Self::flatten_toc(toc);

        let mut itemrefs = Vec::new();
        for entry in flat_toc {
            // 查找对应的 manifest 项
            let href = opf_document::relative_href(
                entry
                    .content_src
                    .split('#')
                    .next()
                    .unwrap_or(&entry.content_src),
            );
            let manifest_item = manifest.items.iter().find(|item| item.href == href);

            if let Some(item) = manifest_item {
                itemrefs.push(StandardSpineItemref {
                    idref: item.id.clone(),
                    linear: Some(true),
                });
            }
        }

        opf_document::write_spine_order(base_path, &StandardSpine { itemrefs })
    }

    /// 完整同步（更新所有文件）
    pub fn sync_toc_changes(epub_id: &str, toc: &[TocChapterDto]) -> Result<(), RefactorError> {
        let storage_path = crate::epub::storage::get_epub_storage_path(epub_id);

        // 加载 manifest
        let manifest = Self::load_manifest(&storage_path)?;

        // 更新 nav.xhtml
        Self::update_nav_file(toc, &storage_path)?;

        // 更新 toc.ncx
        Self::update_ncx_file(toc, &storage_path)?;

        // 更新 spine 顺序
        Self::update_spine_order(toc, &manifest, &storage_path)?;

        resource_index::refresh_resource_index(epub_id, &storage_path)?;

        Ok(())
    }

    /// 加载 manifest
    fn load_manifest(base_path: &str) -> Result<StandardManifest, RefactorError> {
        opf_document::load_opf_document(base_path, "unknown").map(|document| document.manifest)
    }

    /// 展平目录结构 - 直接返回新的 Vec 而非引用
    fn flatten_toc(toc: &[TocChapterDto]) -> Vec<TocChapterDto> {
        let mut result = Vec::new();
        Self::flatten_toc_recursive(toc, &mut result);
        result
    }

    fn flatten_toc_recursive(toc: &[TocChapterDto], result: &mut Vec<TocChapterDto>) {
        for entry in toc {
            result.push(entry.clone());
            Self::flatten_toc_recursive(&entry.children, result);
        }
    }

    fn toc_to_navigation(toc: &[TocChapterDto]) -> Navigation {
        fn convert(entries: &[TocChapterDto]) -> Vec<TocEntry> {
            entries
                .iter()
                .map(|entry| TocEntry {
                    id: entry.id.clone(),
                    label: entry.label.clone(),
                    content_src: entry.content_src.clone(),
                    level: entry.level,
                    children: convert(&entry.children),
                })
                .collect()
        }

        Navigation { toc: convert(toc) }
    }

    /// 提取 XML 属性
    fn extract_attribute(content: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = content.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = content[value_start..].find('"') {
                return Some(content[value_start..value_start + end].to_string());
            }
        }
        None
    }

    /// 提取 nav label
    fn extract_nav_label(content: &str) -> Option<String> {
        if let Some(text_start) = content.find("<text>") {
            if let Some(text_end) = content[text_start..].find("</text>") {
                let text = &content[text_start + 6..text_start + text_end];
                return Some(text.trim().to_string());
            }
        }
        None
    }

    /// 提取 NCX label
    fn extract_ncx_label(content: &str) -> Option<String> {
        if let Some(text_start) = content.find("<text>") {
            if let Some(text_end) = content[text_start..].find("</text>") {
                let text = &content[text_start + 6..text_start + text_end];
                return Some(text.trim().to_string());
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

    /// 提取 content src (从 navPoint)
    fn extract_content_src(content: &str) -> Option<String> {
        Self::extract_attribute(content, "src")
    }

    /// 查找匹配的闭合标签
    fn find_matching_close(content: &str, open_tag: &str, close_tag: &str) -> Option<usize> {
        let mut depth = 0;
        let mut search_pos = 0;

        while search_pos < content.len() {
            let remaining = &content[search_pos..];
            let next_open = remaining.find(open_tag);
            let next_close = remaining.find(close_tag);

            match (next_open, next_close) {
                (Some(open_pos), Some(close_pos)) if open_pos < close_pos => {
                    depth += 1;
                    search_pos += open_pos + open_tag.len();
                }
                (Some(_), Some(close_pos)) | (None, Some(close_pos)) => {
                    depth -= 1;
                    let close_end = search_pos + close_pos + close_tag.len();
                    if depth == 0 {
                        return Some(close_end);
                    }
                    search_pos = close_end;
                }
                (Some(open_pos), None) => {
                    depth += 1;
                    search_pos += open_pos + open_tag.len();
                }
                (None, None) => break,
            }
        }

        None
    }
}

fn starts_with_ignore_ascii_case_at(content: &str, index: usize, needle: &str) -> bool {
    let Some(end) = index.checked_add(needle.len()) else {
        return false;
    };

    content
        .get(index..end)
        .map(|slice| slice.eq_ignore_ascii_case(needle))
        .unwrap_or(false)
}

/// 辅助函数：更新目录条目
pub fn update_toc_entry(
    epub_id: &str,
    entry_id: &str,
    new_label: Option<&str>,
    new_content_src: Option<&str>,
) -> Result<(), RefactorError> {
    let storage_path = crate::epub::storage::get_epub_storage_path(epub_id);

    // 加载现有目录
    let mut toc = TocManager::load_toc_from_storage(&storage_path)?;

    // 更新条目
    let updated = update_toc_entry_recursive(&mut toc, entry_id, new_label, new_content_src);

    if !updated {
        return Err(RefactorError::InvalidStructure(format!(
            "TOC entry not found: {}",
            entry_id
        )));
    }

    // 同步更改
    TocManager::sync_toc_changes(epub_id, &toc)
}

/// 递归更新目录条目
fn update_toc_entry_recursive(
    entries: &mut [TocChapterDto],
    entry_id: &str,
    new_label: Option<&str>,
    new_content_src: Option<&str>,
) -> bool {
    for entry in entries.iter_mut() {
        if entry.id == entry_id {
            if let Some(label) = new_label {
                entry.label = label.to_string();
            }
            if let Some(content_src) = new_content_src {
                entry.content_src = content_src.to_string();
                entry.file_path = Some(TocManager::parse_content_src(content_src, ""));
            }
            return true;
        }
        if update_toc_entry_recursive(&mut entry.children, entry_id, new_label, new_content_src) {
            return true;
        }
    }
    false
}

/// 辅助函数：更新目录顺序
pub fn update_toc_order(epub_id: &str, new_order: &[TocOrderDto]) -> Result<(), RefactorError> {
    // 将前端传输的顺序转换为 TocChapterDto
    let mut order_counter = 0usize;
    let toc: Vec<TocChapterDto> = convert_order_to_toc(new_order, 0, &mut order_counter);

    // 同步更改
    TocManager::sync_toc_changes(epub_id, &toc)
}

/// 将 TocOrderDto 转换为 TocChapterDto
fn convert_order_to_toc(
    order: &[TocOrderDto],
    level: usize,
    order_counter: &mut usize,
) -> Vec<TocChapterDto> {
    order
        .iter()
        .map(|o| {
            let order = *order_counter;
            *order_counter += 1;

            TocChapterDto {
                id: o.id.clone(),
                label: o.label.clone(),
                content_src: o.content_src.clone(),
                file_path: Some(TocManager::parse_content_src(&o.content_src, "")),
                file_paths: Vec::new(),
                file_name: None,
                file_names: Vec::new(),
                anchor: None,
                level,
                order,
                children: convert_order_to_toc(&o.children, level + 1, order_counter),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_content_src_preserves_standard_directories() {
        assert_eq!(
            TocManager::parse_content_src("Text/part1/chapter.xhtml", ""),
            "OEBPS/Text/part1/chapter.xhtml"
        );
        assert_eq!(
            TocManager::parse_content_src("Images/cover.jpg", ""),
            "OEBPS/Images/cover.jpg"
        );
        assert_eq!(
            TocManager::parse_content_src("chapter.xhtml#frag", ""),
            "OEBPS/Text/chapter.xhtml"
        );
    }

    #[test]
    fn parses_ncx_sibling_navpoints_once() {
        let content = r#"
        <ncx>
          <navMap>
            <navPoint id="chapter-1">
              <navLabel><text>第一章</text></navLabel>
              <content src="Text/chapter1.xhtml"/>
            </navPoint>
            <navPoint id="chapter-2">
              <navLabel><text>第二章</text></navLabel>
              <content src="Text/chapter2.xhtml"/>
            </navPoint>
          </navMap>
        </ncx>
        "#;

        let mut entries = Vec::new();
        let mut order_counter = 0usize;
        TocManager::parse_ncx_navmap(content, 0, &mut entries, &mut order_counter, "");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "第一章");
        assert_eq!(entries[1].label, "第二章");
        assert!(entries[0].children.is_empty());
        assert!(entries[1].children.is_empty());
    }

    #[test]
    fn parses_nav_sibling_items_once() {
        let content = r#"
        <ol class="toc">
          <li><a href="Text/chapter1.xhtml">第一章</a></li>
          <li><a href="Text/chapter2.xhtml">第二章</a></li>
        </ol>
        "#;

        let mut entries = Vec::new();
        let mut order_counter = 0usize;
        TocManager::parse_ol_elements(content, 0, &mut entries, &mut order_counter, "");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "第一章");
        assert_eq!(entries[1].label, "第二章");
        assert!(entries[0].children.is_empty());
        assert!(entries[1].children.is_empty());
    }

    #[test]
    fn normalizes_duplicate_nav_sources_to_single_toc() {
        let entries = vec![
            test_toc_entry("nav-html-1", "第一章", "episode1.xhtml", 0),
            test_toc_entry("nav-html-2", "第二章", "episode2.xhtml", 1),
            test_toc_entry("ncx-1", "第一章", "Text/episode1.xhtml", 2),
            test_toc_entry("ncx-2", "第二章", "Text/episode2.xhtml", 3),
        ];

        let normalized = TocManager::normalize_toc_entries(entries);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].id, "ncx-1");
        assert_eq!(normalized[0].order, 0);
        assert_eq!(normalized[1].id, "ncx-2");
        assert_eq!(normalized[1].order, 1);
    }

    fn test_toc_entry(id: &str, label: &str, content_src: &str, order: usize) -> TocChapterDto {
        TocChapterDto {
            id: id.to_string(),
            label: label.to_string(),
            content_src: content_src.to_string(),
            file_path: Some(TocManager::parse_content_src(content_src, "")),
            file_paths: Vec::new(),
            file_name: None,
            file_names: Vec::new(),
            anchor: None,
            level: 0,
            order,
            children: Vec::new(),
        }
    }
}
