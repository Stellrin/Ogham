use super::models::{
    ImageProcessFailure, ImageProcessProgress, ImageProcessResult, RefactorError,
};
use super::storage::get_epub_storage_path;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// 下载图片并返回二进制内容和 MIME 类型
pub fn download_image(url: &str) -> Result<(Vec<u8>, String), RefactorError> {
    // 使用带请求头的 client
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| RefactorError::IoError(format!("创建 HTTP 客户端失败: {}", e)))?;

    let response = client.get(url)
        .send()
        .map_err(|e| RefactorError::IoError(format!("下载图片失败: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(RefactorError::IoError(format!(
            "HTTP 请求失败: {}",
            status
        )));
    }

    // 获取 Content-Type 头
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // 读取图片内容
    let bytes = response
        .bytes()
        .map_err(|e| RefactorError::IoError(format!("读取图片数据失败: {}", e)))?
        .to_vec();

    Ok((bytes, content_type))
}

/// 根据 MIME 类型获取文件扩展名
fn get_extension_from_mime(mime: &str) -> String {
    match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "jpg",
    }
    .to_string()
}

fn is_chapter_href(href: &str) -> bool {
    let normalized = href.replace('\\', "/").to_lowercase();
    (normalized.starts_with("text/") || normalized.contains("/text/"))
        && (normalized.ends_with(".xhtml") || normalized.ends_with(".html"))
}

/// 添加图片到 OPF manifest
fn add_image_to_manifest(storage_path: &str, image_filename: &str, mime_type: &str) -> Result<(), RefactorError> {
    let opf_path = Path::new(storage_path).join("OEBPS/content.opf");

    // 读取现有 OPF 文件
    let opf_content = fs::read_to_string(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取 OPF 文件: {}", e)))?;

    let href = format!("Images/{}", image_filename);

    if opf_content.contains(&format!("href=\"{}\"", href)) {
        return Ok(());
    }

    // 生成唯一 ID
    let stem = image_filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(image_filename);

    let mut item_id = format!("img_{}", stem);
    let mut suffix = 1;
    while opf_content.contains(&format!("id=\"{}\"", item_id)) {
        item_id = format!("img_{}_{}", stem, suffix);
        suffix += 1;
    }

    // 构建新的 manifest 项
    let new_item = format!(
        "    <item id=\"{}\" href=\"{}\" media-type=\"{}\" />",
        item_id, href, mime_type
    );

    // 插入到 manifest 开始标签之后
    let marker = "<manifest>";
    if let Some(pos) = opf_content.find(marker) {
        let insert_pos = pos + marker.len();
        let mut new_opf = opf_content[..insert_pos].to_string();
        new_opf.push('\n');
        new_opf.push_str(&new_item);
        new_opf.push_str(&opf_content[insert_pos..]);

        // 写回 OPF 文件
        fs::write(&opf_path, &new_opf)
            .map_err(|e| RefactorError::IoError(format!("无法写入 OPF 文件: {}", e)))?;
    }

    Ok(())
}

fn next_image_index(images_dir: &Path) -> u32 {
    if !images_dir.exists() {
        return 1;
    }

    let mut max_index = 0u32;
    if let Ok(entries) = fs::read_dir(images_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(number_part) = stem.strip_prefix("image_") {
                    if let Ok(value) = number_part.parse::<u32>() {
                        if value > max_index {
                            max_index = value;
                        }
                    }
                }
            }
        }
    }

    max_index + 1
}

fn collect_chapters_by_spine(storage_path: &str) -> Result<Vec<String>, RefactorError> {
    let opf_path = Path::new(storage_path).join("OEBPS/content.opf");
    let opf_content = fs::read_to_string(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法读取 OPF 文件: {}", e)))?;

    let item_re = Regex::new(r#"<item\s+[^>]*id="([^"]+)"[^>]*href="([^"]+)"[^>]*/?>"#)
        .map_err(|e| RefactorError::IoError(format!("解析 manifest 规则失败: {}", e)))?;
    let itemref_re = Regex::new(r#"<itemref\s+[^>]*idref="([^"]+)"[^>]*/?>"#)
        .map_err(|e| RefactorError::IoError(format!("解析 spine 规则失败: {}", e)))?;

    let mut id_to_href: HashMap<String, String> = HashMap::new();
    for caps in item_re.captures_iter(&opf_content) {
        let id = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let href = caps
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or("")
            .replace('\\', "/");
        if !id.is_empty() && !href.is_empty() {
            id_to_href.insert(id, href);
        }
    }

    let mut chapters = Vec::new();
    for caps in itemref_re.captures_iter(&opf_content) {
        let idref = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if let Some(href) = id_to_href.get(idref) {
            if is_chapter_href(href) {
                chapters.push(format!("OEBPS/{}", href));
            }
        }
    }

    if chapters.is_empty() {
        let text_dir = Path::new(storage_path).join("OEBPS/Text");
        if !text_dir.exists() {
            return Err(RefactorError::IoError("Text 目录不存在".to_string()));
        }

        let mut fallback_files = fs::read_dir(&text_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("xhtml"))
                    .unwrap_or(false)
            })
            .map(|entry| {
                let filename = entry.file_name().to_string_lossy().to_string();
                format!("OEBPS/Text/{}", filename)
            })
            .collect::<Vec<_>>();
        fallback_files.sort();
        return Ok(fallback_files);
    }

    Ok(chapters)
}

fn emit_progress<F>(
    progress_callback: &mut F,
    task_id: &str,
    chapter_path: String,
    current_chapter_index: u32,
    total_chapters: u32,
    image_url: Option<String>,
    processed_unique_images: u32,
    total_unique_images: u32,
    successful_images: u32,
    failed_images: u32,
    skipped_duplicates: u32,
    stage: &str,
    message: &str,
) where
    F: FnMut(ImageProcessProgress),
{
    progress_callback(ImageProcessProgress {
        task_id: task_id.to_string(),
        chapter_path,
        current_chapter_index,
        total_chapters,
        image_url,
        processed_unique_images,
        total_unique_images,
        successful_images,
        failed_images,
        skipped_duplicates,
        stage: stage.to_string(),
        message: message.to_string(),
    });
}

/// 处理整本 EPUB 中所有章节的图片链接，返回处理结果统计
pub fn process_all_images<F>(epub_id: &str, task_id: &str, mut progress_callback: F) -> Result<ImageProcessResult, RefactorError>
where
    F: FnMut(ImageProcessProgress),
{
    let storage_path = get_epub_storage_path(epub_id);
    let chapter_files = collect_chapters_by_spine(&storage_path)?;
    let image_link_re = Regex::new(r"<p>&lt;图片&gt;(https?://[^<]+)</p>")
        .map_err(|e| RefactorError::IoError(format!("图片匹配规则初始化失败: {}", e)))?;

    let total_chapters = chapter_files.len() as u32;
    let mut chapter_contents: Vec<(String, String, Vec<(String, String)>)> = Vec::new();
    let mut detected_raw_matches = 0u32;
    let mut unique_urls = HashSet::new();

    for chapter_path in &chapter_files {
        let full_chapter_path = Path::new(&storage_path).join(chapter_path);
        let content = fs::read_to_string(&full_chapter_path)
            .map_err(|e| RefactorError::IoError(format!("无法读取章节文件 {}: {}", chapter_path, e)))?;

        let mut matches = Vec::new();
        for caps in image_link_re.captures_iter(&content) {
            let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
            let url = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            if !matched.is_empty() && !url.is_empty() {
                matches.push((matched, url.clone()));
                detected_raw_matches += 1;
                unique_urls.insert(url);
            }
        }

        chapter_contents.push((chapter_path.clone(), content, matches));
    }

    let total_unique_images = unique_urls.len() as u32;

    let images_dir = Path::new(&storage_path).join("OEBPS/Images");
    fs::create_dir_all(&images_dir)?;

    let mut next_index = next_image_index(&images_dir);
    let mut url_to_local_path: HashMap<String, String> = HashMap::new();

    let mut processed_chapters = 0u32;
    let mut processed_unique_images = 0u32;
    let mut successful_images = 0u32;
    let mut failed_images = 0u32;
    let mut skipped_duplicates = 0u32;
    let mut inserted_images = 0u32;
    let mut failures: Vec<ImageProcessFailure> = Vec::new();

    for (chapter_idx, (chapter_path, content, matches)) in chapter_contents.into_iter().enumerate() {
        let mut new_content = content.clone();
        let chapter_index = chapter_idx as u32 + 1;

        emit_progress(
            &mut progress_callback,
            task_id,
            chapter_path.clone(),
            chapter_index,
            total_chapters,
            None,
            processed_unique_images,
            total_unique_images,
            successful_images,
            failed_images,
            skipped_duplicates,
            "chapter_start",
            "开始处理章节",
        );

        for (matched_str, url) in matches {
            if let Some(existing_path) = url_to_local_path.get(&url) {
                let img_tag = format!("<p><img src=\"{}\" alt=\"图片\" /></p>", existing_path);
                new_content = new_content.replacen(&matched_str, &img_tag, 1);
                skipped_duplicates += 1;
                inserted_images += 1;

                emit_progress(
                    &mut progress_callback,
                    task_id,
                    chapter_path.clone(),
                    chapter_index,
                    total_chapters,
                    Some(url.clone()),
                    processed_unique_images,
                    total_unique_images,
                    successful_images,
                    failed_images,
                    skipped_duplicates,
                    "reuse",
                    "复用已下载图片",
                );

                continue;
            }

            emit_progress(
                &mut progress_callback,
                task_id,
                chapter_path.clone(),
                chapter_index,
                total_chapters,
                Some(url.clone()),
                processed_unique_images,
                total_unique_images,
                successful_images,
                failed_images,
                skipped_duplicates,
                "download",
                "下载图片中",
            );

            match download_image(&url) {
                Ok((image_bytes, mime_type)) => {
                    let ext = get_extension_from_mime(&mime_type);
                    let image_filename = format!("image_{:06}.{}", next_index, ext);
                    next_index += 1;

                    let full_image_path = images_dir.join(&image_filename);
                    let image_path_in_zip = format!("OEBPS/Images/{}", image_filename);

                    let mut file = File::create(&full_image_path)
                        .map_err(|e| RefactorError::IoError(format!("无法创建图片文件: {}", e)))?;
                    file.write_all(&image_bytes)
                        .map_err(|e| RefactorError::IoError(format!("无法写入图片: {}", e)))?;

                    add_image_to_manifest(&storage_path, &image_filename, &mime_type)?;

                    let img_tag = format!("<p><img src=\"{}\" alt=\"图片\" /></p>", image_path_in_zip);
                    new_content = new_content.replacen(&matched_str, &img_tag, 1);

                    url_to_local_path.insert(url.clone(), image_path_in_zip);

                    processed_unique_images += 1;
                    successful_images += 1;
                    inserted_images += 1;

                    emit_progress(
                        &mut progress_callback,
                        task_id,
                        chapter_path.clone(),
                        chapter_index,
                        total_chapters,
                        Some(url.clone()),
                        processed_unique_images,
                        total_unique_images,
                        successful_images,
                        failed_images,
                        skipped_duplicates,
                        "saved",
                        "图片已保存并插入章节",
                    );
                }
                Err(err) => {
                    processed_unique_images += 1;
                    failed_images += 1;
                    failures.push(ImageProcessFailure {
                        chapter_path: chapter_path.clone(),
                        image_url: url.clone(),
                        error: err.to_string(),
                    });

                    emit_progress(
                        &mut progress_callback,
                        task_id,
                        chapter_path.clone(),
                        chapter_index,
                        total_chapters,
                        Some(url.clone()),
                        processed_unique_images,
                        total_unique_images,
                        successful_images,
                        failed_images,
                        skipped_duplicates,
                        "failed",
                        "图片下载失败",
                    );
                }
            }
        }

        if new_content != content {
            let full_chapter_path = Path::new(&storage_path).join(&chapter_path);
            fs::write(&full_chapter_path, &new_content)
                .map_err(|e| RefactorError::IoError(format!("无法写入章节文件: {}", e)))?;
        }

        processed_chapters += 1;

        emit_progress(
            &mut progress_callback,
            task_id,
            chapter_path,
            chapter_index,
            total_chapters,
            None,
            processed_unique_images,
            total_unique_images,
            successful_images,
            failed_images,
            skipped_duplicates,
            "chapter_done",
            "章节处理完成",
        );
    }

    Ok(ImageProcessResult {
        task_id: task_id.to_string(),
        total_chapters,
        processed_chapters,
        detected_raw_matches,
        detected_unique_urls: total_unique_images,
        successful_images,
        failed_images,
        skipped_duplicates,
        inserted_images,
        failures,
    })
}
