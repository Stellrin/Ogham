use super::models::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// 生成 toc.ncx 文件（EPUB 2.0 兼容）
pub fn generate_toc_ncx(navigation: &Navigation, base_path: &str) -> Result<(), RefactorError> {
    let ncx_content = render_toc_ncx(navigation)?;

    let ncx_path = Path::new(base_path).join("OEBPS/toc.ncx");
    let mut file = File::create(&ncx_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 NCX 文件: {}", e)))?;

    file.write_all(ncx_content.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 NCX 文件: {}", e)))?;

    Ok(())
}

/// 生成 nav.xhtml 文件（EPUB 3.0 导航）
pub fn generate_nav_xhtml(navigation: &Navigation, base_path: &str) -> Result<(), RefactorError> {
    let nav_content = render_nav_xhtml(navigation)?;

    let nav_path = Path::new(base_path).join("OEBPS/nav.xhtml");
    let mut file = File::create(&nav_path)
        .map_err(|e| RefactorError::IoError(format!("无法创建 NAV 文件: {}", e)))?;

    file.write_all(nav_content.as_bytes())
        .map_err(|e| RefactorError::IoError(format!("无法写入 NAV 文件: {}", e)))?;

    Ok(())
}

/// 渲染完整的 toc.ncx 内容
pub fn render_toc_ncx(navigation: &Navigation) -> Result<String, RefactorError> {
    build_ncx_xml(navigation)
}

/// 渲染完整的 nav.xhtml 内容
pub fn render_nav_xhtml(navigation: &Navigation) -> Result<String, RefactorError> {
    build_nav_xhtml(navigation)
}

/// 构建 NCX XML 内容
fn build_ncx_xml(navigation: &Navigation) -> Result<String, RefactorError> {
    let max_depth = calculate_max_depth(&navigation.toc);

    let xml = format!(
        r####"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="ogham-book-id"/>
    <meta name="dtb:depth" content="{}"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle>
    <text>目录</text>
  </docTitle>
  <navMap>
    {}
  </navMap>
</ncx>
"####,
        max_depth,
        build_ncx_nav_points(&navigation.toc, 1, 0),
    );

    Ok(xml)
}

/// 构建 NCX navPoints
fn build_ncx_nav_points(entries: &[TocEntry], start_id: usize, level: usize) -> String {
    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let id = start_id + i + 1;
            let play_order = id;
            let indent = "  ".repeat(level + 1);
            let indent_2 = format!("{}  ", indent);
            let indent_4 = format!("{}    ", indent);

            format!(
                "{}<navPoint id=\"navPoint-{}\" playOrder=\"{}\">\n{}<navLabel>\n{}<text>{}</text>\n{}</navLabel>\n{}<content src=\"{}\"/>\n{}\n{}</navPoint>",
                indent,
                id,
                play_order,
                indent_2,
                indent_4,
                escape_xml(&entry.label),
                indent_2,
                indent_2,
                escape_xml(&entry.content_src),
                indent,
                if entry.children.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", build_ncx_nav_points(&entry.children, id * 1000, level + 2))
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 构建 nav.xhtml 内容
fn build_nav_xhtml(navigation: &Navigation) -> Result<String, RefactorError> {
    let xml = format!(
        r####"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
  <head>
    <title>目录</title>
    <style type="text/css">
      nav {{
        display: block;
      }}
      nav ol {{
        list-style-type: none;
        padding-left: 0;
      }}
      nav ol.toc {{
        padding-left: 2em;
      }}
      nav li {{
        margin: 0.5em 0;
      }}
      nav a {{
        text-decoration: none;
        color: inherit;
      }}
      nav a:hover {{
        text-decoration: underline;
      }}
    </style>
  </head>
  <body>
    <nav epub:type="toc">
      <h1>目录</h1>
      <ol class="toc">
        {}
      </ol>
    </nav>
  </body>
</html>
"####,
        build_nav_xhtml_items(&navigation.toc)
    );

    Ok(xml)
}

/// 构建 nav.xhtml 列表项
fn build_nav_xhtml_items(entries: &[TocEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            let children_html = if entry.children.is_empty() {
                String::new()
            } else {
                format!(
                    "\n        <ol class=\"toc\">\n          {}\n        </ol>",
                    build_nav_xhtml_items(&entry.children)
                )
            };

            format!(
                "        <li>\n          <a href=\"{}\">{}</a>{}\n        </li>",
                escape_xml(&entry.content_src),
                escape_xml(&entry.label),
                if children_html.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", children_html)
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 计算目录最大深度
fn calculate_max_depth(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| {
            if entry.children.is_empty() {
                1
            } else {
                1 + calculate_max_depth(&entry.children)
            }
        })
        .max()
        .unwrap_or(1)
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
        assert_eq!(
            escape_xml("Chapter 1: The Beginning"),
            "Chapter 1: The Beginning"
        );
        assert_eq!(escape_xml("A & B"), "A &amp; B");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_calculate_max_depth() {
        let entries = vec![
            TocEntry {
                id: "1".to_string(),
                label: "Chapter 1".to_string(),
                content_src: "ch1.xhtml".to_string(),
                level: 0,
                children: vec![TocEntry {
                    id: "1-1".to_string(),
                    label: "Section 1.1".to_string(),
                    content_src: "ch1-1.xhtml".to_string(),
                    level: 1,
                    children: vec![],
                }],
            },
            TocEntry {
                id: "2".to_string(),
                label: "Chapter 2".to_string(),
                content_src: "ch2.xhtml".to_string(),
                level: 0,
                children: vec![],
            },
        ];

        assert_eq!(calculate_max_depth(&entries), 2);
    }

    #[test]
    fn test_build_nav_xhtml_items() {
        let entries = vec![TocEntry {
            id: "1".to_string(),
            label: "Chapter 1".to_string(),
            content_src: "Text/chapter1.xhtml".to_string(),
            level: 0,
            children: vec![],
        }];

        let result = build_nav_xhtml_items(&entries);
        assert!(result.contains("Chapter 1"));
        assert!(result.contains("Text/chapter1.xhtml"));
        assert!(result.contains("<li>"));
        assert!(result.contains("<a href="));
    }

    #[test]
    fn test_build_ncx_xml() {
        let navigation = Navigation {
            toc: vec![TocEntry {
                id: "1".to_string(),
                label: "Chapter 1".to_string(),
                content_src: "Text/chapter1.xhtml".to_string(),
                level: 0,
                children: vec![],
            }],
        };

        let result = build_ncx_xml(&navigation).unwrap();
        assert!(result.contains("<ncx"));
        assert!(result.contains("<navMap>"));
        assert!(result.contains("Chapter 1"));
        assert!(result.contains("Text/chapter1.xhtml"));
    }
}
