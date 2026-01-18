use super::models::*;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipArchive;
use regex::Regex;

/// 从原始 EPUB 复制和整理文件到标准目录
pub fn copy_and_organize_files(
    source_path: &str,
    storage_path: &str,
    analysis: &EpubStructureAnalysis,
) -> Result<(), RefactorError> {
    // 打开源 EPUB 文件
    let file = File::open(source_path)
        .map_err(|e| RefactorError::IoError(format!("无法打开源文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| RefactorError::ParseError(format!("无法打开 ZIP 归档: {}", e)))?;

    // 复制章节文件
    for chapter in &analysis.chapters {
        extract_and_copy_file(&mut archive, &chapter.original_path, storage_path, &chapter.standard_path)?;
    }

    // 复制样式文件
    for style in &analysis.resources.styles {
        extract_and_copy_file(&mut archive, &style.original_path, storage_path, &style.standard_path)?;
    }

    // 复制图片文件
    for image in &analysis.resources.images {
        extract_and_copy_file(&mut archive, &image.original_path, storage_path, &image.standard_path)?;
    }

    // 复制字体文件
    for font in &analysis.resources.fonts {
        extract_and_copy_file(&mut archive, &font.original_path, storage_path, &font.standard_path)?;
    }

    // 复制其他文件
    for misc in &analysis.resources.misc {
        extract_and_copy_file(&mut archive, &misc.original_path, storage_path, &misc.standard_path)?;
    }

    Ok(())
}

/// 从 ZIP 中提取并复制文件
fn extract_and_copy_file(
    archive: &mut ZipArchive<File>,
    original_path: &str,
    storage_path: &str,
    standard_path: &str,
) -> Result<(), RefactorError> {
    // 尝试从 ZIP 中提取文件
    let mut source_file = match archive.by_name(original_path) {
        Ok(file) => file,
        Err(_) => {
            // 文件不存在，跳过
            return Ok(());
        }
    };

    // 读取文件内容
    let mut buffer = Vec::new();
    source_file.read_to_end(&mut buffer)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {}: {}", original_path, e)))?;

    // 创建目标路径
    let target_path = Path::new(storage_path).join(standard_path);

    // 确保父目录存在
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| RefactorError::IoError(format!("无法创建目录 {:?}: {}", parent, e)))?;
    }

    // 写入文件
    let mut target_file = File::create(&target_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建文件 {:?}: {}", target_path, e)))?;

    target_file.write_all(&buffer)
        .map_err(|e| RefactorError::IoError(format!("无法写入文件 {:?}: {}", target_path, e)))?;

    Ok(())
}

/// 更新文件中的资源引用
pub fn update_resource_references(
    storage_path: &str,
    analysis: &EpubStructureAnalysis,
) -> Result<(), RefactorError> {
    // 更新章节文件中的引用
    for chapter in &analysis.chapters {
        update_file_references(
            storage_path,
            &chapter.original_path,
            &chapter.standard_path,
            &analysis.file_map,
        )?;
    }

    // 更新样式文件中的引用
    for style in &analysis.resources.styles {
        update_file_references(
            storage_path,
            &style.original_path,
            &style.standard_path,
            &analysis.file_map,
        )?;
    }

    Ok(())
}

/// 更新单个文件中的资源引用
fn update_file_references(
    storage_path: &str,
    original_path: &str,
    standard_path: &str,
    file_map: &FileMap,
) -> Result<(), RefactorError> {
    let full_path = Path::new(storage_path).join(standard_path);

    // 读取文件内容
    let mut content = fs::read_to_string(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", full_path, e)))?;

    // 获取原始文件所在目录（用于解析引用）
    let original_dir = if let Some(parent) = Path::new(original_path).parent() {
        parent.to_string_lossy().to_string()
    } else {
        String::new()
    };

    // 获取重构后文件所在目录（用于计算新相对路径）
    let standard_dir = if let Some(parent) = Path::new(standard_path).parent() {
        parent.to_string_lossy().to_string()
    } else {
        String::new()
    };

    // 重写资源路径
    content = rewrite_resource_paths(&content, &original_dir, &standard_dir, file_map);

    // 写回文件
    fs::write(&full_path, content)
        .map_err(|e| RefactorError::IoError(format!("无法写入文件 {:?}: {}", full_path, e)))?;

    Ok(())
}

/// 重写资源路径
fn rewrite_resource_paths(
    content: &str,
    original_dir: &str,
    standard_dir: &str,
    file_map: &FileMap,
) -> String {
    // 匹配 src="" 和 href="" 属性
    let src_regex = Regex::new(r#"(src|href|xlink:href)="([^"]+)""#)
        .unwrap_or_else(|_| Regex::new(r#"src="[^"]*""#).unwrap());

    let result = src_regex.replace_all(content, |caps: &regex::Captures| {
        let attr_name = &caps[1];
        let original_ref = &caps[2];

        // 跳过已经是 data: URI 的
        if original_ref.starts_with("data:") {
            return format!(r#"{}="{}""#, attr_name, original_ref);
        }

        // 跳过外部链接和锚点
        if original_ref.starts_with("http://")
            || original_ref.starts_with("https://")
            || original_ref.starts_with('#')
        {
            return format!(r#"{}="{}""#, attr_name, original_ref);
        }

        // 使用原始目录解析相对引用，得到原始完整路径
        let original_full_path = resolve_relative_path(original_dir, original_ref);

        // 在 file_map 中查找对应的标准路径
        let standard_target_path = find_standard_path(&original_full_path, file_map);

        if let Some(target_path) = standard_target_path {
            // 计算从重构后目录到目标标准路径的相对路径
            let relative_path = calculate_relative_path(standard_dir, &target_path);
            format!(r#"{}="{}""#, attr_name, relative_path)
        } else {
            // 没有找到映射，保持原样
            format!(r#"{}="{}""#, attr_name, original_ref)
        }
    });

    result.to_string()
}

/// 查找标准路径（尝试多种路径格式）
fn find_standard_path(path: &str, file_map: &FileMap) -> Option<String> {
    // 首先直接查找
    if let Some(std_path) = file_map.original_to_standard.get(path) {
        return Some(std_path.clone());
    }

    // 尝试去掉 OEBPS/ 前缀
    if let Some(rest) = path.strip_prefix("OEBPS/") {
        if let Some(std_path) = file_map.original_to_standard.get(rest) {
            return Some(std_path.clone());
        }
    }

    // 尝试添加 OEBPS/ 前缀
    let with_prefix = format!("OEBPS/{}", path);
    if let Some(std_path) = file_map.original_to_standard.get(&with_prefix) {
        return Some(std_path.clone());
    }

    // 如果路径本身就是标准路径，直接返回
    if path.starts_with("OEBPS/") {
        return Some(path.to_string());
    }

    None
}

/// 解析相对路径
fn resolve_relative_path(base: &str, relative: &str) -> String {
    let mut base_parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();

    for part in &path_parts {
        if *part == ".." {
            base_parts.pop();
        } else if !part.starts_with('.') {
            base_parts.push(part);
        }
    }

    if base_parts.is_empty() {
        path_parts.join("/")
    } else {
        base_parts.join("/")
    }
}

/// 计算相对路径（从源目录到目标文件）
fn calculate_relative_path(from_dir: &str, to_path: &str) -> String {
    // 解析源目录和目标路径
    let from_parts: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to_path.split('/').filter(|s| !s.is_empty()).collect();

    // 找到公共前缀
    let mut common_len = 0;
    for (i, (f, t)) in from_parts.iter().zip(to_parts.iter()).enumerate() {
        if f == t {
            common_len = i + 1;
        } else {
            break;
        }
    }

    // 计算需要向上多少级
    let up_levels = from_parts.len() - common_len;

    // 构建相对路径
    let mut result = Vec::new();

    // 添加向上路径
    for _ in 0..up_levels {
        result.push("..");
    }

    // 添加向下路径
    for part in &to_parts[common_len..] {
        result.push(*part);
    }

    // 特殊情况：同一目录
    if result.is_empty() {
        // 返回文件名
        return to_parts.last().unwrap_or(&"").to_string();
    }

    result.join("/")
}

/// 从重构后的 EPUB 中读取文件内容
pub fn read_refactored_file(
    storage_path: &str,
    file_path: &str,
) -> Result<String, RefactorError> {
    let full_path = Path::new(storage_path).join(file_path);

    fs::read_to_string(&full_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取文件 {:?}: {}", full_path, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(
            resolve_relative_path("OEBPS/Text", "../Images/cover.jpg"),
            "OEBPS/Images/cover.jpg"
        );

        assert_eq!(
            resolve_relative_path("OEBPS/Text", "chapter2.xhtml"),
            "OEBPS/Text/chapter2.xhtml"
        );
    }

    #[test]
    fn test_calculate_relative_path() {
        assert_eq!(
            calculate_relative_path("OEBPS/Text", "OEBPS/Images/cover.jpg"),
            "../Images/cover.jpg"
        );

        assert_eq!(
            calculate_relative_path("OEBPS/Text", "OEBPS/Styles/style.css"),
            "../Styles/style.css"
        );

        assert_eq!(
            calculate_relative_path("OEBPS", "OEBPS/Text/chapter1.xhtml"),
            "Text/chapter1.xhtml"
        );
    }

    #[test]
    fn test_rewrite_resource_paths() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        // 原始路径：OEBPS/Images/cover.jpg
        // 标准路径：OEBPS/Images/cover.jpg (相同)
        file_map.original_to_standard.insert(
            "OEBPS/Images/cover.jpg".to_string(),
            "OEBPS/Images/cover.jpg".to_string(),
        );

        // 测试：文件从 OEBPS/Text 移动到 OEBPS/Text
        // 原始引用：../Images/cover.jpg (相对于 OEBPS/Text)
        let html = r#"<html><body><img src="../Images/cover.jpg" /></body></html>"#;
        let result = rewrite_resource_paths(html, "OEBPS/Text", "OEBPS/Text", &file_map);

        assert!(result.contains("src=\"../Images/cover.jpg\""));
    }

    #[test]
    fn test_rewrite_same_dir_image_ref() {
        let mut file_map = FileMap {
            original_to_standard: HashMap::new(),
            standard_to_original: HashMap::new(),
        };

        // 原始：cover.xhtml 在 OEBPS/，引用 cover.png (同目录)
        // 重构后：cover.xhtml 在 OEBPS/Text/，图片在 OEBPS/Images/
        file_map.original_to_standard.insert(
            "cover.png".to_string(),
            "OEBPS/Images/cover.png".to_string(),
        );

        let html = r#"<html><body><svg><image xlink:href="cover.png"/></svg></body></html>"#;
        // 原始目录：OEBPS (空字符串表示根目录)
        // 重构后目录：OEBPS/Text
        let result = rewrite_resource_paths(html, "", "OEBPS/Text", &file_map);

        // 应该更新为 ../Images/cover.png
        assert!(result.contains("xlink:href=\"../Images/cover.png\""));
    }
}
