use super::models::*;
use epub::doc::EpubDoc;
use std::collections::HashMap;
use std::io::BufReader;

/// EPUB 解析错误类型
#[derive(Debug)]
pub enum EpubParseError {
    InvalidZip(String),
    XmlError(String),
    MissingFile(String),
    ContainerError(String),
    OpfError(String),
}

impl std::fmt::Display for EpubParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpubParseError::InvalidZip(msg) => write!(f, "无效的 ZIP 文件: {}", msg),
            EpubParseError::XmlError(msg) => write!(f, "XML 解析错误: {}", msg),
            EpubParseError::MissingFile(msg) => write!(f, "缺少必需文件: {}", msg),
            EpubParseError::ContainerError(msg) => write!(f, "容器解析错误: {}", msg),
            EpubParseError::OpfError(msg) => write!(f, "OPF 解析错误: {}", msg),
        }
    }
}

impl std::error::Error for EpubParseError {}

impl From<epub::doc::DocError> for EpubParseError {
    fn from(err: epub::doc::DocError) -> Self {
        EpubParseError::InvalidZip(err.to_string())
    }
}

impl From<zip::result::ZipError> for EpubParseError {
    fn from(err: zip::result::ZipError) -> Self {
        EpubParseError::InvalidZip(err.to_string())
    }
}

/// 解析 EPUB 主入口函数
pub fn parse_epub(file_path: &str) -> Result<(ParsedEpub, String), EpubParseError> {
    // 使用 epub 库打开文档
    let doc = EpubDoc::new(file_path)
        .map_err(|e| EpubParseError::InvalidZip(e.to_string()))?;

    // 获取 OPF 路径
    let opf_path = doc.root_file.to_string_lossy().to_string();

    // 提取元数据
    let metadata = extract_metadata(&doc);

    // 提取 manifest
    let manifest = extract_manifest(&doc);

    // 提取 spine
    let spine = extract_spine(&doc);

    // 提取目录（TOC）
    let toc = extract_toc(&doc)?;

    Ok((
        ParsedEpub {
            metadata,
            manifest,
            spine,
            table_of_contents: toc,
        },
        opf_path,
    ))
}

/// 提取元数据
fn extract_metadata(doc: &EpubDoc<BufReader<std::fs::File>>) -> EpubMetadata {
    EpubMetadata {
        title: doc
            .mdata("title")
            .map(|m| m.value.clone())
            .unwrap_or_else(|| "Unknown Title".to_string()),
        author: doc.mdata("creator").map(|m| m.value.clone()),
        language: doc.mdata("language").map(|m| m.value.clone()),
        identifier: doc
            .mdata("identifier")
            .map(|m| m.value.clone())
            .unwrap_or_else(|| "".to_string()),
    }
}

/// 提取 manifest
fn extract_manifest(doc: &EpubDoc<BufReader<std::fs::File>>) -> Manifest {
    let mut manifest_items = HashMap::new();

    // 遍历所有资源
    for (id, resource) in doc.resources.iter() {
        let properties = if resource.properties.is_some() {
            resource.properties.as_ref().map(|p| {
                if p.is_empty() {
                    None
                } else {
                    Some(p.split_whitespace().map(String::from).collect())
                }
            }).flatten()
        } else {
            None
        };

        manifest_items.insert(
            id.clone(),
            ManifestItem {
                id: id.clone(),
                href: resource.path.to_string_lossy().to_string(),
                media_type: resource.mime.clone(),
                properties,
            },
        );
    }

    Manifest {
        items: manifest_items,
    }
}

/// 提取 spine
fn extract_spine(doc: &EpubDoc<BufReader<std::fs::File>>) -> Spine {
    let spine_itemrefs: Vec<SpineItemref> = doc
        .spine
        .iter()
        .map(|spine_item| SpineItemref {
            idref: spine_item.idref.clone(),
            linear: Some(spine_item.linear),
        })
        .collect();

    Spine {
        itemrefs: spine_itemrefs,
    }
}

/// 提取目录（TOC）
fn extract_toc(doc: &EpubDoc<BufReader<std::fs::File>>) -> Result<Option<Toc>, EpubParseError> {
    // epub 库的 toc 是 Vec<NavPoint>
    if !doc.toc.is_empty() {
        let nav_points = convert_epub_toc(&doc.toc);
        return Ok(Some(Toc { nav_points }));
    }

    Ok(None)
}

/// 转换 epub 库的 TOC 结构到我们的 NavPoint
fn convert_epub_toc(epub_toc: &[epub::doc::NavPoint]) -> Vec<NavPoint> {
    epub_toc
        .iter()
        .map(|epub_nav| NavPoint {
            id: format!("nav-point-{:?}", epub_nav.play_order),
            label: epub_nav.label.clone(),
            content_src: epub_nav.content.to_string_lossy().to_string(),
            children: convert_epub_toc(&epub_nav.children),
        })
        .collect()
}

/// 获取 OPF 文件所在目录
pub fn get_opf_base_path(opf_path: &str) -> String {
    if let Some(last_slash) = opf_path.rfind('/') {
        opf_path[..last_slash].to_string()
    } else {
        String::new()
    }
}

/// 获取章节内容（通过资源 ID）
pub fn get_chapter_content(
    doc: &mut EpubDoc<BufReader<std::fs::File>>,
    id: &str,
) -> Result<String, EpubParseError> {
    // 使用 epub 库的 get_resource_str 方法
    match doc.get_resource_str(id) {
        Some((content, _mime)) => Ok(content),
        None => Err(EpubParseError::MissingFile(format!(
            "章节不存在: {}",
            id
        ))),
    }
}
