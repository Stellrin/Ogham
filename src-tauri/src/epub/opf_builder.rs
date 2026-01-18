use super::models::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// 生成 content.opf 文件
pub fn generate_content_opf(
    metadata: &EpubMetadata,
    manifest: &StandardManifest,
    spine: &StandardSpine,
    base_path: &str,
) -> Result<(), RefactorError> {
    let opf_content = build_opf_xml(metadata, manifest, spine)?;

    let opf_path = Path::new(base_path).join("OEBPS/content.opf");
    let mut file = File::create(&opf_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 OPF 文件: {}", e)))?;

    file.write_all(opf_content.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 OPF 文件: {}", e)))?;

    Ok(())
}

/// 构建 OPF XML 内容
fn build_opf_xml(
    metadata: &EpubMetadata,
    manifest: &StandardManifest,
    spine: &StandardSpine,
) -> Result<String, RefactorError> {
    let unique_id = "book-id".to_string();
    let modified_date = get_current_date();

    let xml = format!(r##"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="{}" xml:lang="{}">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="{}">{}</dc:identifier>
    <dc:title>{}</dc:title>
    {}
    <dc:language>{}</dc:language>
    <meta property="dcterms:modified">{}</meta>
  </metadata>

  <manifest>
    {}
  </manifest>

  <spine>
    {}
  </spine>
</package>
"##,
        unique_id,
        metadata.language.as_deref().unwrap_or("en"),
        unique_id,
        escape_xml(&metadata.identifier),
        escape_xml(&metadata.title),
        if let Some(ref author) = metadata.author {
            format!("<dc:creator>{}</dc:creator>", escape_xml(author))
        } else {
            String::new()
        },
        metadata.language.as_deref().unwrap_or("en"),
        modified_date,
        build_manifest_items(manifest),
        build_spine_items(spine),
    );

    Ok(xml)
}

/// 构建 manifest 项目
fn build_manifest_items(manifest: &StandardManifest) -> String {
    manifest.items
        .iter()
        .map(|item| {
            let properties_attr = if let Some(ref props) = item.properties {
                if !props.is_empty() {
                    format!(" properties=\"{}\"", props.join(" "))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            format!(
                "    <item id=\"{}\" href=\"{}\" media-type=\"{}\"{} />",
                escape_xml(&item.id),
                escape_xml(&item.href),
                escape_xml(&item.media_type),
                properties_attr
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 构建 spine 项目
fn build_spine_items(spine: &StandardSpine) -> String {
    spine.itemrefs
        .iter()
        .map(|itemref| {
            let linear_attr = itemref.linear
                .map(|l| format!(" linear=\"{}\"", if l { "yes" } else { "no" }))
                .unwrap_or_default();

            format!(
                "    <itemref idref=\"{}\"{} />",
                escape_xml(&itemref.idref),
                linear_attr
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 获取当前日期（ISO 8601 格式）
fn get_current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();

    let secs = duration.as_secs();
    let datetime = chrono_timestamp_to_datetime(secs);

    // 格式化为 YYYY-MM-DDTHH:MM:SSZ
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        datetime.year,
        datetime.month,
        datetime.day,
        datetime.hour,
        datetime.minute,
        datetime.second
    )
}

/// 简单的时间戳转日期时间
struct DateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

fn chrono_timestamp_to_datetime(secs: u64) -> DateTime {
    // 简化计算：从 1970-01-01 开始
    let days_since_epoch = (secs / 86400) as i32;
    let seconds_of_day = (secs % 86400) as u32;

    // 简化：假设每年 365 天（忽略闰年）
    let year = 1970 + (days_since_epoch / 365);
    let day_of_year = (days_since_epoch % 365) as u32;

    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

/// XML 转义
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello & World"), "Hello &amp; World");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quote\""), "&quot;quote&quot;");
    }

    #[test]
    fn test_build_manifest_items() {
        let manifest = StandardManifest {
            items: vec![
                StandardManifestItem {
                    id: "chapter1".to_string(),
                    href: "Text/chapter1.xhtml".to_string(),
                    media_type: "application/xhtml+xml".to_string(),
                    properties: None,
                },
            ],
        };

        let result = build_manifest_items(&manifest);
        assert!(result.contains("id=\"chapter1\""));
        assert!(result.contains("href=\"Text/chapter1.xhtml\""));
    }

    #[test]
    fn test_get_current_date() {
        let date = get_current_date();
        assert!(date.contains('T'));
        assert!(date.contains('Z'));
        assert!(date.starts_with(|c: char| c.is_ascii_digit()));
    }
}
