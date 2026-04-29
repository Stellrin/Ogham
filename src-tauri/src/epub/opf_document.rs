use super::epub_path;
use super::models::*;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct OpfDocument {
    pub metadata: EpubMetadata,
    pub manifest: StandardManifest,
    pub spine: StandardSpine,
}

pub fn load_opf_document(
    storage_path: &str,
    fallback_identifier: &str,
) -> Result<OpfDocument, RefactorError> {
    let opf_path = Path::new(storage_path).join("OEBPS/content.opf");
    let content = fs::read_to_string(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取 content.opf: {}", e)))?;

    parse_opf_document(&content, fallback_identifier)
}

pub fn parse_opf_document(
    content: &str,
    fallback_identifier: &str,
) -> Result<OpfDocument, RefactorError> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut metadata = EpubMetadata {
        title: fallback_identifier.to_string(),
        author: None,
        language: None,
        identifier: fallback_identifier.to_string(),
    };
    let mut manifest_items = Vec::new();
    let mut spine_itemrefs = Vec::new();
    let mut current_metadata_tag: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = element_name(&element);
                match name.as_str() {
                    "dc:identifier" | "identifier" | "dc:title" | "title" | "dc:creator"
                    | "creator" | "dc:language" | "language" => {
                        current_metadata_tag = Some(name);
                    }
                    "item" => {
                        if let Some(item) = parse_manifest_item(&element)? {
                            manifest_items.push(item);
                        }
                    }
                    "itemref" => {
                        if let Some(itemref) = parse_spine_itemref(&element)? {
                            spine_itemrefs.push(itemref);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                let name = element_name(&element);
                match name.as_str() {
                    "item" => {
                        if let Some(item) = parse_manifest_item(&element)? {
                            manifest_items.push(item);
                        }
                    }
                    "itemref" => {
                        if let Some(itemref) = parse_spine_itemref(&element)? {
                            spine_itemrefs.push(itemref);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(tag_name) = current_metadata_tag.as_deref() {
                    let value = String::from_utf8_lossy(text.as_ref()).trim().to_string();
                    if !value.is_empty() {
                        match tag_name {
                            "dc:identifier" | "identifier" => metadata.identifier = value,
                            "dc:title" | "title" => metadata.title = value,
                            "dc:creator" | "creator" => metadata.author = Some(value),
                            "dc:language" | "language" => metadata.language = Some(value),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).to_string();
                if current_metadata_tag.as_deref() == Some(name.as_str()) {
                    current_metadata_tag = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RefactorError::XmlError(format!(
                    "解析 content.opf 失败: {}",
                    e
                )));
            }
            _ => {}
        }
    }

    Ok(OpfDocument {
        metadata,
        manifest: StandardManifest {
            items: manifest_items,
        },
        spine: StandardSpine {
            itemrefs: spine_itemrefs,
        },
    })
}

pub fn write_spine_order(storage_path: &str, spine: &StandardSpine) -> Result<(), RefactorError> {
    let opf_path = Path::new(storage_path).join("OEBPS/content.opf");
    let content = fs::read_to_string(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取 content.opf: {}", e)))?;

    let rendered_spine = render_spine(spine);
    let updated_content =
        replace_xml_section(&content, "spine", &rendered_spine).ok_or_else(|| {
            RefactorError::InvalidStructure("content.opf 缺少 spine 节点".to_string())
        })?;

    fs::write(&opf_path, updated_content)
        .map_err(|e| RefactorError::IoError(format!("无法写入 content.opf: {}", e)))?;

    Ok(())
}

pub fn append_manifest_item(
    storage_path: &str,
    item: &StandardManifestItem,
) -> Result<(), RefactorError> {
    let opf_path = Path::new(storage_path).join("OEBPS/content.opf");
    let content = fs::read_to_string(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取 content.opf: {}", e)))?;

    let document = parse_opf_document(&content, "unknown")?;
    if document
        .manifest
        .items
        .iter()
        .any(|existing| existing.href == item.href)
    {
        return Ok(());
    }

    let manifest_start = content.find("<manifest").ok_or_else(|| {
        RefactorError::InvalidStructure("content.opf 缺少 manifest 节点".to_string())
    })?;
    let insert_pos = content[manifest_start..]
        .find('>')
        .map(|pos| manifest_start + pos + 1)
        .ok_or_else(|| RefactorError::InvalidStructure("manifest 起始标签不完整".to_string()))?;

    let mut updated_content = String::new();
    updated_content.push_str(&content[..insert_pos]);
    updated_content.push('\n');
    updated_content.push_str(&render_manifest_item(item));
    updated_content.push_str(&content[insert_pos..]);

    fs::write(&opf_path, updated_content)
        .map_err(|e| RefactorError::IoError(format!("无法写入 content.opf: {}", e)))?;

    Ok(())
}

pub fn relative_href(path: &str) -> String {
    epub_path::to_opf_href(path)
}

pub fn make_unique_manifest_id(manifest: &StandardManifest, preferred_id: &str) -> String {
    let mut candidate = sanitize_manifest_id(preferred_id);
    let base = candidate.clone();
    let mut suffix = 1usize;

    while manifest.items.iter().any(|item| item.id == candidate) {
        candidate = format!("{}_{}", base, suffix);
        suffix += 1;
    }

    candidate
}

fn parse_manifest_item(
    element: &BytesStart<'_>,
) -> Result<Option<StandardManifestItem>, RefactorError> {
    let id = attr_value(element, "id")?.unwrap_or_default();
    let href = attr_value(element, "href")?.unwrap_or_default();
    let media_type = attr_value(element, "media-type")?.unwrap_or_default();
    let properties = attr_value(element, "properties")?
        .filter(|props| !props.trim().is_empty())
        .map(|props| props.split_whitespace().map(String::from).collect());

    if id.is_empty() || href.is_empty() {
        return Ok(None);
    }

    Ok(Some(StandardManifestItem {
        id,
        href,
        media_type,
        properties,
    }))
}

fn parse_spine_itemref(
    element: &BytesStart<'_>,
) -> Result<Option<StandardSpineItemref>, RefactorError> {
    let idref = attr_value(element, "idref")?.unwrap_or_default();
    let linear = attr_value(element, "linear")?.map(|value| value != "no");

    if idref.is_empty() {
        return Ok(None);
    }

    Ok(Some(StandardSpineItemref { idref, linear }))
}

fn attr_value(element: &BytesStart<'_>, attr_name: &str) -> Result<Option<String>, RefactorError> {
    for attr in element.attributes().with_checks(false) {
        let attr = attr.map_err(|e| RefactorError::XmlError(e.to_string()))?;
        if attr.key.as_ref() == attr_name.as_bytes() {
            return Ok(Some(
                String::from_utf8_lossy(attr.value.as_ref()).to_string(),
            ));
        }
    }

    Ok(None)
}

fn element_name(element: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(element.name().as_ref()).to_string()
}

fn render_spine(spine: &StandardSpine) -> String {
    let items = spine
        .itemrefs
        .iter()
        .map(|itemref| {
            let linear = itemref
                .linear
                .map(|value| format!(" linear=\"{}\"", if value { "yes" } else { "no" }))
                .unwrap_or_default();

            format!(
                "    <itemref idref=\"{}\"{} />",
                escape_xml(&itemref.idref),
                linear
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("  <spine toc=\"ncx\">\n{}\n  </spine>", items)
}

fn render_manifest_item(item: &StandardManifestItem) -> String {
    let properties = item
        .properties
        .as_ref()
        .filter(|values| !values.is_empty())
        .map(|values| format!(" properties=\"{}\"", escape_xml(&values.join(" "))))
        .unwrap_or_default();

    format!(
        "    <item id=\"{}\" href=\"{}\" media-type=\"{}\"{} />",
        escape_xml(&item.id),
        escape_xml(&item.href),
        escape_xml(&item.media_type),
        properties
    )
}

fn sanitize_manifest_id(value: &str) -> String {
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
        "item".to_string()
    } else {
        id
    }
}

fn replace_xml_section(content: &str, tag_name: &str, replacement: &str) -> Option<String> {
    let start_tag = format!("<{}", tag_name);
    let end_tag = format!("</{}>", tag_name);
    let start = content.find(&start_tag)?;
    let end = content[start..].find(&end_tag)? + start + end_tag.len();

    Some(format!(
        "{}{}{}",
        &content[..start],
        replacement,
        &content[end..]
    ))
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_opf_document_reads_manifest_spine_and_metadata() {
        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package>
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">id-1</dc:identifier>
    <dc:title>Title</dc:title>
    <dc:creator>Author</dc:creator>
    <dc:language>zh-CN</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav" />
    <item id="c1" href="Text/c1.xhtml" media-type="application/xhtml+xml" />
  </manifest>
  <spine toc="ncx">
    <itemref idref="c1" linear="yes" />
  </spine>
</package>"#;

        let document = parse_opf_document(opf, "fallback").unwrap();

        assert_eq!(document.metadata.title, "Title");
        assert_eq!(document.manifest.items.len(), 2);
        assert_eq!(document.spine.itemrefs.len(), 1);
        assert_eq!(document.spine.itemrefs[0].idref, "c1");
    }

    #[test]
    fn replace_spine_keeps_surrounding_opf_content() {
        let opf =
            "<package><metadata></metadata><spine><itemref idref=\"old\" /></spine></package>";
        let spine = StandardSpine {
            itemrefs: vec![StandardSpineItemref {
                idref: "new".to_string(),
                linear: Some(true),
            }],
        };

        let updated = replace_xml_section(opf, "spine", &render_spine(&spine)).unwrap();

        assert!(updated.contains("idref=\"new\""));
        assert!(updated.contains("<metadata></metadata>"));
    }

    #[test]
    fn make_unique_manifest_id_suffixes_existing_ids() {
        let manifest = StandardManifest {
            items: vec![StandardManifestItem {
                id: "img_cover".to_string(),
                href: "Images/cover.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                properties: None,
            }],
        };

        assert_eq!(
            make_unique_manifest_id(&manifest, "img_cover"),
            "img_cover_1"
        );
    }
}
