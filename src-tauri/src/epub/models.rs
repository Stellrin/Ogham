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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChapterInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub order: usize,
}

// ============================================================================
// 重构系统数据结构
// ============================================================================

/// 重构错误类型
#[derive(Debug)]
pub enum RefactorError {
    IoError(String),
    ParseError(String),
    InvalidStructure(String),
    MissingFile(String),
    XmlError(String),
}

impl std::fmt::Display for RefactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefactorError::IoError(msg) => write!(f, "IO 错误: {}", msg),
            RefactorError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            RefactorError::InvalidStructure(msg) => write!(f, "无效结构: {}", msg),
            RefactorError::MissingFile(msg) => write!(f, "文件缺失: {}", msg),
            RefactorError::XmlError(msg) => write!(f, "XML 错误: {}", msg),
        }
    }
}

impl std::error::Error for RefactorError {}

impl From<std::io::Error> for RefactorError {
    fn from(err: std::io::Error) -> Self {
        RefactorError::IoError(err.to_string())
    }
}

/// 重构后的完整 EPUB 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefactoredEpub {
    pub epub_id: String,
    pub metadata: EpubMetadata,
    pub manifest: StandardManifest,
    pub spine: StandardSpine,
    pub navigation: Navigation,
    pub storage_path: String,
    pub file_map: FileMap,
    pub created_at: u64,
}

/// 标准化清单
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardManifest {
    pub items: Vec<StandardManifestItem>,
}

/// 标准化清单项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardManifestItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub properties: Option<Vec<String>>,
}

/// 标准化阅读顺序
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardSpine {
    pub itemrefs: Vec<StandardSpineItemref>,
}

/// 标准化阅读顺序项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardSpineItemref {
    pub idref: String,
    pub linear: Option<bool>,
}

/// 导航结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Navigation {
    pub toc: Vec<TocEntry>,
}

/// 目录条目
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TocEntry {
    pub id: String,
    pub label: String,
    pub content_src: String,
    pub level: usize,
    pub children: Vec<TocEntry>,
}

/// 资源集合（分类）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceCollection {
    pub styles: Vec<ResourceFile>,
    pub images: Vec<ResourceFile>,
    pub fonts: Vec<ResourceFile>,
    pub misc: Vec<ResourceFile>,
}

/// 资源文件
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceFile {
    pub id: String,
    pub original_path: String,
    pub standard_path: String,
    pub media_type: String,
}

/// 扩展章节信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtendedChapterInfo {
    pub id: String,
    pub original_path: String,
    pub standard_path: String,
    pub title: Option<String>,
    pub order: usize,
    pub media_type: String,
}

/// 文件路径映射
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMap {
    /// 原始路径 -> 标准路径
    pub original_to_standard: HashMap<String, String>,
    /// 标准路径 -> 原始路径
    pub standard_to_original: HashMap<String, String>,
}

/// 特殊文件
#[derive(Debug, Clone)]
pub struct SpecialFiles {
    pub cover: Option<String>,
    pub nav: Option<String>,
    pub ncx: Option<String>,
}

/// EPUB 结构分析结果
#[derive(Debug, Clone)]
pub struct EpubStructureAnalysis {
    pub chapters: Vec<ExtendedChapterInfo>,
    pub resources: ResourceCollection,
    pub special_files: SpecialFiles,
    pub file_map: FileMap,
}

/// 重构结果（返回给前端）
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactoredEpubResult {
    pub epub_id: String,
    pub metadata: EpubMetadata,
    pub structure: StandardEpubStructure,
    pub storage_path: String,
}

/// 标准化 EPUB 结构（返回给前端）
#[derive(Debug, Serialize, Deserialize)]
pub struct StandardEpubStructure {
    pub chapters: Vec<StandardChapter>,
    pub styles: Vec<String>,
    pub images: Vec<String>,
    pub fonts: Vec<String>,
    pub navigation: Vec<NavigationEntry>,
}

/// 标准化章节
#[derive(Debug, Serialize, Deserialize)]
pub struct StandardChapter {
    pub id: String,
    pub original_filename: String,
    pub standard_path: String,
    pub title: Option<String>,
    pub order: usize,
}

/// 导航条目
#[derive(Debug, Serialize, Deserialize)]
pub struct NavigationEntry {
    pub id: String,
    pub label: String,
    pub content_src: String,
    pub level: usize,
    pub children: Vec<NavigationEntry>,
}

// ============================================================================
// 目录管理 DTO 结构
// ============================================================================

/// 目录章节 DTO（用于前后端传输）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TocChapterDto {
    pub id: String,
    pub label: String,
    pub content_src: String,
    pub file_path: Option<String>,
    pub level: usize,
    pub order: usize,
    pub children: Vec<TocChapterDto>,
}

/// 目录顺序更新 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct TocOrderDto {
    pub id: String,
    pub label: String,
    pub content_src: String,
    pub children: Vec<TocOrderDto>,
}

/// 目录条目更新请求
#[derive(Debug, Serialize, Deserialize)]
pub struct TocUpdateRequest {
    pub entry_id: String,
    pub new_label: Option<String>,
    pub new_content_src: Option<String>,
}

/// 图片批量处理结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageProcessResult {
    pub task_id: String,
    pub total_chapters: u32,
    pub processed_chapters: u32,
    pub detected_raw_matches: u32,
    pub detected_unique_urls: u32,
    pub successful_images: u32,
    pub failed_images: u32,
    pub skipped_duplicates: u32,
    pub inserted_images: u32,
    pub failures: Vec<ImageProcessFailure>,
}

/// 单张图片处理失败详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageProcessFailure {
    pub chapter_path: String,
    pub image_url: String,
    pub error: String,
}

/// 图片处理进度事件
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageProcessProgress {
    pub task_id: String,
    pub chapter_path: String,
    pub current_chapter_index: u32,
    pub total_chapters: u32,
    pub image_url: Option<String>,
    pub processed_unique_images: u32,
    pub total_unique_images: u32,
    pub successful_images: u32,
    pub failed_images: u32,
    pub skipped_duplicates: u32,
    pub stage: String,
    pub message: String,
}
