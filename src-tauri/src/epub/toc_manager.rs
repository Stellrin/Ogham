use crate::epub::epub_path;
use crate::epub::models::{
    Navigation, RefactorError, StandardManifest, StandardSpine, StandardSpineItemref,
    TocChapterDto, TocEntry, TocOrderDto,
};
use crate::epub::opf_document;
use crate::epub::resource_index;
use std::fs;
use std::path::Path;

/// 目录管理器 - 负责解析、更新、同步目录与 OPF/导航文件
pub struct TocManager;

impl TocManager {
    /// 加载并解析目录到文件的映射
    pub fn load_toc_with_file_mapping(
        epub_id: &str,
        base_path: &str,
    ) -> Result<Vec<TocChapterDto>, RefactorError> {
        let storage_path = crate::epub::storage::get_epub_storage_path(epub_id);
        let nav_path = Path::new(&storage_path).join("OEBPS/nav.xhtml");
        let ncx_path = Path::new(&storage_path).join("OEBPS/toc.ncx");

        // 优先尝试加载 nav.xhtml
        if nav_path.exists() {
            return Self::load_from_nav(&nav_path.to_string_lossy(), base_path);
        }

        // 回退到 toc.ncx
        if ncx_path.exists() {
            return Self::load_from_ncx(&ncx_path.to_string_lossy(), base_path);
        }

        Err(RefactorError::MissingFile(
            "No navigation file found (nav.xhtml or toc.ncx)".to_string(),
        ))
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

        // 查找所有 nav 元素（特别是 epub:type="toc" 的）
        Self::parse_nav_epub3_elements(content, 0, &mut entries, &mut order_counter, base_path);

        Ok(entries)
    }

    /// 递归解析 EPUB 3.0 nav 元素（HTML 格式）
    fn parse_nav_epub3_elements(
        content: &str,
        level: usize,
        entries: &mut Vec<TocChapterDto>,
        order_counter: &mut usize,
        base_path: &str,
    ) {
        // 查找所有 <nav> 元素
        let mut search_pos = 0;
        while let Some(nav_start) = content[search_pos..].find("<nav") {
            let nav_start = search_pos + nav_start;
            // 找到 nav 元素的结束位置
            let nav_end =
                match Self::find_matching_close_html(&content[nav_start..], "<nav", "</nav>") {
                    Some(end) => nav_start + end,
                    None => {
                        search_pos = nav_start + 4;
                        continue;
                    }
                };

            let nav_content = &content[nav_start..nav_end];

            // 检查是否是目录 nav（epub:type="toc"）或包含 <ol> 的 nav
            if nav_content.contains("epub:type=\"toc\"") || nav_content.contains("<ol>") {
                // 解析 <ol>/<li>/<a> 结构
                Self::parse_ol_elements(nav_content, level, entries, order_counter, base_path);
            }

            search_pos = nav_end;
        }
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
            let lower_slice = content[search_pos..].to_lowercase();

            if lower_slice.starts_with(&open_tag_lower) {
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
            } else if lower_slice.starts_with(&close_tag_lower) {
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

        // 查找所有 navPoint
        let mut search_pos = 0;
        while let Some(point_start) = nav_map_content[search_pos..].find("<navPoint") {
            let point_start = search_pos + point_start;
            let point_end = Self::find_matching_close(
                &nav_map_content[point_start..],
                "<navPoint",
                "</navPoint>",
            )
            .map(|end| point_start + end)
            .unwrap_or(nav_map_content.len());

            let nav_point_content = &nav_map_content[point_start..point_end];

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
            Self::parse_nav_elements(
                nav_point_content,
                level + 1,
                &mut children,
                order_counter,
                base_path,
            );

            let order = *order_counter;
            *order_counter += 1;

            entries.push(TocChapterDto {
                id,
                label,
                content_src,
                file_path: Some(file_path),
                level,
                order,
                children,
            });

            search_pos = point_end;
        }
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
            Self::parse_ncx_navmap(
                nav_point_content,
                level + 1,
                &mut children,
                order_counter,
                base_path,
            );

            let order = *order_counter;
            *order_counter += 1;

            entries.push(TocChapterDto {
                id,
                label,
                content_src,
                file_path: Some(file_path),
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
            if let Some(open_pos) = content[search_pos..].find(open_tag) {
                depth += 1;
                search_pos += open_pos + open_tag.len();
            } else if let Some(close_pos) = content[search_pos..].find(close_tag) {
                depth -= 1;
                if depth == 0 {
                    return Some(search_pos + close_pos + close_tag.len());
                }
                search_pos += close_pos + close_tag.len();
            } else {
                break;
            }
        }

        None
    }
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
    let mut toc = TocManager::load_toc_with_file_mapping(epub_id, &storage_path)?;

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
}
