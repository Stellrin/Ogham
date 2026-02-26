use super::models::*;
use std::fs::{self, File};
use std::io::{Write, Read, Seek};
use std::path::Path;
use zip::{ZipWriter, write::FileOptions};

/// 获取 EPUB 存储路径
pub fn get_epub_storage_path(epub_id: &str) -> String {
    // 使用系统临时目录存储 EPUB 文件，避免触发 Tauri 开发监视器
    let temp_dir = std::env::temp_dir();
    let ogham_lib = temp_dir.join("ogham-library").join(epub_id);
    ogham_lib.to_string_lossy().to_string()
}

/// 创建标准目录结构
pub fn create_standard_directories(base_path: &str) -> Result<(), RefactorError> {
    let paths = vec![
        Path::new(base_path).join("OEBPS"),
        Path::new(base_path).join("OEBPS/Text"),
        Path::new(base_path).join("OEBPS/Styles"),
        Path::new(base_path).join("OEBPS/Images"),
        Path::new(base_path).join("OEBPS/Fonts"),
        Path::new(base_path).join("META-INF"),
    ];

    for path in &paths {
        fs::create_dir_all(path)
            .map_err(|e| RefactorError::IoError(format!("无法创建目录 {:?}: {}", path, e)))?;
    }

    // 写入标准支持文件
    write_standard_support_files(base_path)?;

    Ok(())
}

/// 写入标准支持文件（mimetype 和 container.xml）
pub fn write_standard_support_files(base_path: &str) -> Result<(), RefactorError> {
    // 写入 mimetype 文件
    let mimetype_path = Path::new(base_path).join("mimetype");
    let mut mimetype_file = File::create(&mimetype_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 mimetype 文件: {}", e)))?;
    mimetype_file.write_all(b"application/epub+zip")
        .map_err(|e| RefactorError::IoError(format!("无法写入 mimetype 文件: {}", e)))?;

    // 写入 container.xml 文件
    let meta_inf_path = Path::new(base_path).join("META-INF");
    fs::create_dir_all(&meta_inf_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 META-INF 目录: {}", e)))?;

    let container_path = meta_inf_path.join("container.xml");
    let mut container_file = File::create(&container_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 container.xml 文件: {}", e)))?;

    let container_xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"##;

    container_file.write_all(container_xml.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 container.xml 文件: {}", e)))?;

    Ok(())
}

/// 保存重构后的 EPUB 为 ZIP 文件
pub fn save_refactored_epub(
    refactored: &RefactoredEpub,
    epub_id: &str,
) -> Result<String, RefactorError> {
    // 创建输出 ZIP 文件路径（使用系统临时目录）
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("ogham-library").join(format!("{}_refactored.epub", epub_id));
    let output_path_str = output_path.to_string_lossy().to_string();

    // 创建 ZIP 文件
    let file = File::create(&output_path_str)
        .map_err(|e| RefactorError::IoError(format!("无法创建输出文件: {}", e)))?;

    let mut zip = ZipWriter::new(file);

    // 首先添加 mimetype 文件（不压缩）
    let options_mimetype = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", options_mimetype)
        .map_err(|e| RefactorError::IoError(format!("无法添加 mimetype: {}", e)))?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| RefactorError::IoError(format!("无法写入 mimetype: {}", e)))?;

    // 添加 META-INF/container.xml（使用默认压缩）
    zip.start_file("META-INF/container.xml", FileOptions::<()>::default())
        .map_err(|e| RefactorError::IoError(format!("无法添加 container.xml: {}", e)))?;
    let container_xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"##;
    zip.write_all(container_xml.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 container.xml: {}", e)))?;

    // 递归添加 OEBPS 目录下的所有文件
    add_directory_to_zip(&mut zip, &Path::new(&refactored.storage_path).join("OEBPS"), "OEBPS")?;

    // 完成 ZIP 文件
    zip.finish()
        .map_err(|e| RefactorError::IoError(format!("无法完成 ZIP 文件: {}", e)))?;

    Ok(output_path_str)
}

/// 递归添加目录到 ZIP
fn add_directory_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    dir_path: &Path,
    zip_prefix: &str,
) -> Result<(), RefactorError> {
    let entries = fs::read_dir(dir_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取目录 {:?}: {}", dir_path, e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| RefactorError::IoError(format!("无法读取目录项: {}", e)))?;
        let path = entry.path();

        if path.is_dir() {
            // 递归处理子目录
            let new_prefix = format!("{}/{}", zip_prefix, path.file_name().unwrap().to_string_lossy());
            add_directory_to_zip(zip, &path, &new_prefix)?;
        } else {
            // 添加文件到 ZIP
            let file_name = path.file_name().unwrap().to_string_lossy();
            let zip_path = format!("{}/{}", zip_prefix, file_name);

            zip.start_file(&zip_path, FileOptions::<()>::default())
                .map_err(|e| RefactorError::IoError(format!("无法添加文件 {}: {}", zip_path, e)))?;

            let mut file = File::open(&path)
                .map_err(|e| RefactorError::IoError(format!("无法打开文件 {:?}: {}", path, e)))?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", path, e)))?;

            zip.write_all(&buffer)
                .map_err(|e| RefactorError::IoError(format!("无法写入文件 {}: {}", zip_path, e)))?;
        }
    }

    Ok(())
}

/// 从重构后的 EPUB 读取文件
pub fn read_refactored_file(
    storage_path: &str,
    file_path: &str,
) -> Result<Vec<u8>, RefactorError> {
    let full_path = Path::new(storage_path).join(file_path);

    fs::read(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", full_path, e)))
}

/// 清理 EPUB 存储目录
pub fn cleanup_epub_storage(epub_id: &str) -> Result<(), RefactorError> {
    let storage_path = get_epub_storage_path(epub_id);
    let path = Path::new(&storage_path);

    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| RefactorError::IoError(format!("无法删除目录 {:?}: {}", path, e)))?;
    }

    Ok(())
}

/// 清理整个 Ogham 库目录
pub fn cleanup_ogham_library() -> Result<(), RefactorError> {
    let temp_dir = std::env::temp_dir();
    let ogham_lib_path = temp_dir.join("ogham-library");

    if ogham_lib_path.exists() {
        fs::remove_dir_all(&ogham_lib_path)
            .map_err(|e| RefactorError::IoError(format!("无法删除库目录 {:?}: {}", ogham_lib_path, e)))?;
    }

    Ok(())
}

/// 检查 EPUB 是否已存在
pub fn epub_exists(epub_id: &str) -> bool {
    let storage_path = get_epub_storage_path(epub_id);
    Path::new(&storage_path).exists()
}

/// 导出重构后的 EPUB 到指定路径
pub fn export_refactored_epub(
    epub_id: &str,
    export_path: &str,
) -> Result<String, RefactorError> {
    let storage_path = get_epub_storage_path(&epub_id);

    // 检查源文件是否存在
    let source_path = Path::new(&storage_path);
    if !source_path.exists() {
        return Err(RefactorError::IoError(format!(
            "EPUB 不存在，请先重构: {}",
            epub_id
        )));
    }

    // 每次都从存储目录重新构建 ZIP（确保包含转换后的内容）
    rebuild_epub_zip(&storage_path, epub_id, export_path)?;

    Ok(export_path.to_string())
}

/// 从存储目录重新构建 EPUB ZIP 文件
fn rebuild_epub_zip(
    storage_path: &str,
    epub_id: &str,
    export_path: &str,
) -> Result<(), RefactorError> {
    // 创建 ZIP 文件
    let file = File::create(export_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建输出文件: {}", e)))?;

    let mut zip = ZipWriter::new(file);

    // 首先添加 mimetype 文件（不压缩）
    let options_mimetype = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", options_mimetype)
        .map_err(|e| RefactorError::IoError(format!("无法添加 mimetype: {}", e)))?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| RefactorError::IoError(format!("无法写入 mimetype: {}", e)))?;

    // 添加 META-INF/container.xml
    zip.start_file("META-INF/container.xml", FileOptions::<()>::default())
        .map_err(|e| RefactorError::IoError(format!("无法添加 container.xml: {}", e)))?;
    let container_xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"##;
    zip.write_all(container_xml.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 container.xml: {}", e)))?;

    // 递归添加 OEBPS 目录下的所有文件
    let oebps_path = Path::new(storage_path).join("OEBPS");
    add_directory_to_zip(&mut zip, &oebps_path, "OEBPS")?;

    // 完成 ZIP 文件
    zip.finish()
        .map_err(|e| RefactorError::IoError(format!("无法完成 ZIP 文件: {}", e)))?;

    Ok(())
}

/// 获取重构后的 EPUB 文件路径
pub fn get_refactored_epub_path(epub_id: &str) -> String {
    format!("library/{}_refactored.epub", epub_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_epub_storage_path() {
        let path = get_epub_storage_path("test-epub-123");
        let temp_dir = std::env::temp_dir();
        let expected = temp_dir.join("ogham-library").join("test-epub-123");
        assert_eq!(path, expected.to_string_lossy().to_string());
    }

    #[test]
    fn test_write_standard_support_files() -> Result<(), RefactorError> {
        let temp_dir = std::env::temp_dir().join("ogham-test");
        fs::create_dir_all(&temp_dir).unwrap();

        let base_path = temp_dir.to_str().unwrap();
        write_standard_support_files(base_path)?;

        // 检查文件是否存在
        assert!(temp_dir.join("mimetype").exists());
        assert!(temp_dir.join("META-INF/container.xml").exists());

        // 清理
        fs::remove_dir_all(temp_dir).unwrap();

        Ok(())
    }

    #[test]
    fn test_calculate_relative_path() {
        // 这些测试函数实际在 resource_manager.rs 中
        // 这里只是示例
        assert!(true);
    }
}
