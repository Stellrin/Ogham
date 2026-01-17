use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 解析后的 EPUB 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedEpub {
    pub metadata: EpubMetadata,
    pub manifest: Manifest,
    pub spine: Spine,
    pub table_of_contents: Option<Toc>,
}

/// EPUB 元数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpubMetadata {
    pub title: String,
    pub author: Option<String>,
    pub language: Option<String>,
    pub identifier: String,
}

/// 清单（manifest）- 所有文件列表
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub items: HashMap<String, ManifestItem>,
}

/// 清单项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub properties: Option<Vec<String>>, // nav, cover, etc.
}

/// 阅读顺序（spine）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Spine {
    pub itemrefs: Vec<SpineItemref>,
}

/// 阅读顺序项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpineItemref {
    pub idref: String,
    pub linear: Option<bool>,
}

/// 目录结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Toc {
    pub nav_points: Vec<NavPoint>,
}

/// 目录导航点
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NavPoint {
    pub id: String,
    pub label: String,
    pub content_src: String,
    pub children: Vec<NavPoint>,
}

/// 章节内容（包含嵌入的 Base64 资源）
#[derive(Debug, Serialize, Deserialize)]
pub struct ChapterContent {
    pub html: String,
    pub resources: HashMap<String, ResourceData>,
}

/// 资源数据（Base64 编码）
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceData {
    pub mime_type: String,
    pub data: String,
}

/// 标准化 EPUB 结构
#[derive(Debug, Serialize, Deserialize)]
pub struct EpubStructureResult {
    pub metadata: EpubMetadata,
    pub oebps_path: String,
    pub content_opf: String,
    pub toc_ncx: Option<String>,
    pub nav_xhtml: Option<String>,
    pub chapters: Vec<ChapterInfo>,
    pub styles: Vec<String>,
    pub images: Vec<String>,
}

/// 章节信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub order: usize,
}
