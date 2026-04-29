use super::epub_path;
use super::models::*;
use super::opf_document;
use super::storage;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static ATTR_DOUBLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(src|href|xlink:href)\s*=\s*"([^"]+)""#)
        .expect("valid double-quoted attribute reference regex")
});

static ATTR_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(src|href|xlink:href)\s*=\s*'([^']+)'"#)
        .expect("valid single-quoted attribute reference regex")
});

static CSS_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"url\(\s*(['"]?)([^'")]+)(['"]?)\s*\)"#)
        .expect("valid CSS url reference regex")
});

pub fn refresh_resource_index(
    epub_id: &str,
    storage_path: &str,
) -> Result<ResourceIndex, RefactorError> {
    let index = build_resource_index(epub_id, storage_path)?;
    storage::save_resource_index(storage_path, &index)?;
    Ok(index)
}

pub fn load_or_refresh_resource_index(
    epub_id: &str,
    storage_path: &str,
) -> Result<ResourceIndex, RefactorError> {
    if let Some(index) = storage::load_resource_index(storage_path)? {
        if index.epub_id == epub_id {
            return Ok(index);
        }
    }

    refresh_resource_index(epub_id, storage_path)
}

pub fn build_resource_index(
    epub_id: &str,
    storage_path: &str,
) -> Result<ResourceIndex, RefactorError> {
    let opf = opf_document::load_opf_document(storage_path, epub_id)?;
    let mut resources = opf
        .manifest
        .items
        .iter()
        .map(index_item_from_manifest)
        .collect::<Vec<_>>();

    let mut path_to_index = HashMap::new();
    for (index, resource) in resources.iter().enumerate() {
        path_to_index.insert(epub_path::casefold(&resource.path), index);
    }

    let mut references = Vec::new();
    let scan_sources = resources
        .iter()
        .filter(|item| should_scan(item))
        .map(|item| item.path.clone())
        .collect::<Vec<_>>();

    for source_path in scan_sources {
        let full_path = Path::new(storage_path).join(&source_path);
        if !full_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        collect_attribute_references(
            &source_path,
            &content,
            &ATTR_DOUBLE_RE,
            &path_to_index,
            &mut resources,
            &mut references,
        );
        collect_attribute_references(
            &source_path,
            &content,
            &ATTR_SINGLE_RE,
            &path_to_index,
            &mut resources,
            &mut references,
        );
        collect_css_url_references(
            &source_path,
            &content,
            &CSS_URL_RE,
            &path_to_index,
            &mut resources,
            &mut references,
        );
    }

    for resource in &mut resources {
        resource.referenced_by.sort();
        resource.referenced_by.dedup();
    }

    let unresolved_references = references
        .iter()
        .filter(|reference| !reference.is_resolved)
        .cloned()
        .collect();

    Ok(ResourceIndex {
        version: 1,
        epub_id: epub_id.to_string(),
        generated_at: current_timestamp_secs(),
        resources,
        references,
        unresolved_references,
    })
}

fn index_item_from_manifest(item: &StandardManifestItem) -> ResourceIndexItem {
    let href = epub_path::to_opf_href(&item.href);
    let path = epub_path::manifest_path(&href);

    ResourceIndexItem {
        path,
        href,
        manifest_id: Some(item.id.clone()),
        media_type: item.media_type.clone(),
        resource_type: classify_resource_type(item),
        properties: item.properties.clone(),
        referenced_by: Vec::new(),
    }
}

fn classify_resource_type(item: &StandardManifestItem) -> ResourceIndexType {
    let href = epub_path::to_opf_href(&item.href);
    let href_lower = href.to_lowercase();
    let has_nav_property = item
        .properties
        .as_ref()
        .map(|properties| properties.iter().any(|property| property == "nav"))
        .unwrap_or(false);

    if has_nav_property || href_lower == "nav.xhtml" {
        return ResourceIndexType::Nav;
    }

    if item.media_type == "application/x-dtbncx+xml" || href_lower == "toc.ncx" {
        return ResourceIndexType::Ncx;
    }

    if matches!(
        item.media_type.as_str(),
        "application/xhtml+xml" | "text/html" | "application/xml"
    ) && href_lower.starts_with("text/")
    {
        return ResourceIndexType::Chapter;
    }

    if item.media_type == "text/css" {
        return ResourceIndexType::Style;
    }

    if item.media_type.starts_with("image/") {
        return ResourceIndexType::Image;
    }

    if is_font_media_type(&item.media_type) {
        return ResourceIndexType::Font;
    }

    ResourceIndexType::Misc
}

fn is_font_media_type(media_type: &str) -> bool {
    matches!(
        media_type,
        "font/ttf"
            | "font/otf"
            | "font/woff"
            | "font/woff2"
            | "application/font-ttf"
            | "application/font-woff"
            | "application/font-woff2"
    )
}

fn should_scan(resource: &ResourceIndexItem) -> bool {
    epub_path::is_text_like_file(&resource.path)
        || matches!(
            resource.resource_type,
            ResourceIndexType::Chapter
                | ResourceIndexType::Style
                | ResourceIndexType::Nav
                | ResourceIndexType::Ncx
        )
}

fn collect_attribute_references(
    source_path: &str,
    content: &str,
    regex: &Regex,
    path_to_index: &HashMap<String, usize>,
    resources: &mut [ResourceIndexItem],
    references: &mut Vec<ResourceReference>,
) {
    for captures in regex.captures_iter(content) {
        let attribute = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let raw_reference = captures.get(2).map(|m| m.as_str()).unwrap_or("");
        push_reference(
            source_path,
            attribute,
            raw_reference,
            path_to_index,
            resources,
            references,
        );
    }
}

fn collect_css_url_references(
    source_path: &str,
    content: &str,
    regex: &Regex,
    path_to_index: &HashMap<String, usize>,
    resources: &mut [ResourceIndexItem],
    references: &mut Vec<ResourceReference>,
) {
    for captures in regex.captures_iter(content) {
        let raw_reference = captures.get(2).map(|m| m.as_str()).unwrap_or("");
        push_reference(
            source_path,
            "url",
            raw_reference,
            path_to_index,
            resources,
            references,
        );
    }
}

fn push_reference(
    source_path: &str,
    attribute: &str,
    raw_reference: &str,
    path_to_index: &HashMap<String, usize>,
    resources: &mut [ResourceIndexItem],
    references: &mut Vec<ResourceReference>,
) {
    let Some(target_path) = resolve_reference_target(source_path, raw_reference) else {
        return;
    };

    let target_index = path_to_index
        .get(&epub_path::casefold(&target_path))
        .copied();
    let is_resolved = target_index.is_some();

    if let Some(index) = target_index {
        let referenced_by = &mut resources[index].referenced_by;
        if !referenced_by.iter().any(|path| path == source_path) {
            referenced_by.push(source_path.to_string());
        }
    }

    references.push(ResourceReference {
        source_path: source_path.to_string(),
        target_path: Some(target_path),
        raw_reference: raw_reference.trim().to_string(),
        attribute: attribute.to_string(),
        is_resolved,
    });
}

fn resolve_reference_target(source_path: &str, raw_reference: &str) -> Option<String> {
    let trimmed = raw_reference.trim();
    if epub_path::should_skip_reference(trimmed) {
        return None;
    }

    let (path_part, _) = epub_path::split_reference_suffix(trimmed);
    if path_part.is_empty() {
        return None;
    }

    let normalized = epub_path::normalize(path_part);
    if normalized.is_empty() {
        return None;
    }

    if normalized.starts_with("OEBPS/") {
        return Some(normalized);
    }

    if trimmed.starts_with('/') && starts_with_standard_dir(&normalized) {
        return Some(epub_path::manifest_path(&normalized));
    }

    let source_dir = epub_path::parent(source_path);
    let resolved = epub_path::resolve_relative(&source_dir, path_part);

    if !resolved.starts_with("OEBPS/") && starts_with_standard_dir(&resolved) {
        Some(epub_path::manifest_path(&resolved))
    } else {
        Some(resolved)
    }
}

fn starts_with_standard_dir(path: &str) -> bool {
    ["Text/", "Styles/", "Images/", "Fonts/", "Misc/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_index_with_reverse_references_and_unresolved_edges() -> Result<(), RefactorError> {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("ogham-resource-index-test-{}", suffix));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }

        fs::create_dir_all(temp_dir.join("OEBPS/Text")).unwrap();
        fs::create_dir_all(temp_dir.join("OEBPS/Styles")).unwrap();
        fs::create_dir_all(temp_dir.join("OEBPS/Images")).unwrap();

        fs::write(
            temp_dir.join("OEBPS/content.opf"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<package>
  <metadata>
    <dc:identifier>book-id</dc:identifier>
    <dc:title>Book</dc:title>
  </metadata>
  <manifest>
    <item id="c1" href="Text/ch1.xhtml" media-type="application/xhtml+xml" />
    <item id="style" href="Styles/style.css" media-type="text/css" />
    <item id="cover" href="Images/cover.jpg" media-type="image/jpeg" />
    <item id="bg" href="Images/bg.png" media-type="image/png" />
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav" />
  </manifest>
  <spine>
    <itemref idref="c1" />
  </spine>
</package>"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("OEBPS/Text/ch1.xhtml"),
            r#"<html><head><link href="../Styles/style.css" /></head><body><img src="../Images/cover.jpg" /></body></html>"#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("OEBPS/Styles/style.css"),
            r#".cover { background: url("../Images/bg.png"); } .missing { background: url("../Images/missing.png"); }"#,
        )
        .unwrap();
        fs::write(temp_dir.join("OEBPS/Images/cover.jpg"), b"jpg").unwrap();
        fs::write(temp_dir.join("OEBPS/Images/bg.png"), b"png").unwrap();
        fs::write(temp_dir.join("OEBPS/nav.xhtml"), "<nav></nav>").unwrap();

        let index = build_resource_index("book", temp_dir.to_str().unwrap())?;
        let cover = index
            .resources
            .iter()
            .find(|resource| resource.path == "OEBPS/Images/cover.jpg")
            .unwrap();
        assert_eq!(cover.resource_type, ResourceIndexType::Image);
        assert!(cover
            .referenced_by
            .contains(&"OEBPS/Text/ch1.xhtml".to_string()));
        assert_eq!(index.unresolved_references.len(), 1);
        assert_eq!(
            index.unresolved_references[0].target_path.as_deref(),
            Some("OEBPS/Images/missing.png")
        );

        fs::remove_dir_all(temp_dir).unwrap();
        Ok(())
    }
}
