use std::collections::HashSet;

const STANDARD_DIR_PREFIXES: [&str; 5] = ["Text/", "Styles/", "Images/", "Fonts/", "Misc/"];

/// Split a reference into its file path and trailing query/fragment suffix.
pub fn split_reference_suffix(reference: &str) -> (&str, &str) {
    let hash_pos = reference.find('#');
    let query_pos = reference.find('?');

    let split_pos = match (hash_pos, query_pos) {
        (Some(hash), Some(query)) => Some(hash.min(query)),
        (Some(hash), None) => Some(hash),
        (None, Some(query)) => Some(query),
        (None, None) => None,
    };

    if let Some(pos) = split_pos {
        (&reference[..pos], &reference[pos..])
    } else {
        (reference, "")
    }
}

/// Normalize an EPUB internal path and discard query/fragment suffixes.
pub fn normalize(path: &str) -> String {
    let (path_part, _) = split_reference_suffix(path);
    normalize_path_part(path_part)
}

/// Normalize an EPUB reference while preserving query/fragment suffixes.
pub fn normalize_reference(reference: &str) -> String {
    let (path_part, suffix) = split_reference_suffix(reference);
    format!("{}{}", normalize_path_part(path_part), suffix)
}

pub fn strip_oebps_prefix(path: &str) -> String {
    let normalized = normalize(path);
    normalized
        .strip_prefix("OEBPS/")
        .unwrap_or(&normalized)
        .to_string()
}

pub fn to_opf_href(path: &str) -> String {
    strip_oebps_prefix(path)
}

pub fn manifest_path(href: &str) -> String {
    let normalized = normalize(href);
    if normalized.starts_with("OEBPS/") {
        normalized
    } else {
        format!("OEBPS/{}", normalized)
    }
}

pub fn parent(path: &str) -> String {
    let normalized = normalize(path);
    normalized
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

pub fn filename(path: &str) -> String {
    normalize(path)
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

pub fn casefold(path: &str) -> String {
    normalize(path).to_lowercase()
}

pub fn standard_path(target_root: &str, original_href: &str, strip_prefixes: &[&str]) -> String {
    let mut normalized = strip_oebps_prefix(original_href);

    for prefix in strip_prefixes {
        let candidate = format!("{}/", prefix);
        if normalized.starts_with(&candidate) {
            normalized = normalized[candidate.len()..].to_string();
            break;
        }

        let candidate_lower = candidate.to_lowercase();
        if normalized.to_lowercase().starts_with(&candidate_lower) {
            normalized = normalized[candidate.len()..].to_string();
            break;
        }
    }

    format!("OEBPS/{}/{}", target_root, normalized)
}

pub fn allocate_unique_path(path: &str, used_paths: &mut HashSet<String>) -> String {
    let normalized = normalize(path);
    let mut candidate = normalized.clone();
    let mut suffix = 1usize;

    while used_paths.contains(&casefold(&candidate)) {
        candidate = append_filename_suffix(&normalized, suffix);
        suffix += 1;
    }

    used_paths.insert(casefold(&candidate));
    candidate
}

pub fn append_filename_suffix(path: &str, suffix: usize) -> String {
    let normalized = normalize(path);
    let (directory, filename) = normalized
        .rsplit_once('/')
        .map(|(dir, file)| (format!("{}/", dir), file.to_string()))
        .unwrap_or_else(|| (String::new(), normalized));

    let (stem, extension) = filename
        .rsplit_once('.')
        .map(|(stem, ext)| (stem.to_string(), format!(".{}", ext)))
        .unwrap_or_else(|| (filename, String::new()));

    format!("{}{}-{}{}", directory, stem, suffix, extension)
}

pub fn should_skip_reference(reference: &str) -> bool {
    let trimmed = reference.trim();
    let lower = trimmed.to_lowercase();
    trimmed.starts_with('#')
        || lower.starts_with("data:")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("javascript:")
}

pub fn resolve_relative(base_dir: &str, reference: &str) -> String {
    let (path_part, _) = split_reference_suffix(reference);
    let normalized_base = normalize(base_dir);
    let normalized_reference = normalize(path_part);

    if normalized_reference.starts_with("OEBPS/") {
        return normalized_reference;
    }

    let is_root_relative = reference.starts_with('/');
    let mut base_parts: Vec<&str> = if is_root_relative {
        Vec::new()
    } else {
        normalized_base
            .split('/')
            .filter(|part| !part.is_empty())
            .collect()
    };

    for part in normalized_reference
        .split('/')
        .filter(|part| !part.is_empty())
    {
        if part == ".." {
            base_parts.pop();
        } else if part != "." {
            base_parts.push(part);
        }
    }

    base_parts.join("/")
}

pub fn relative_path(from_dir: &str, to_path: &str) -> String {
    let normalized_from = normalize(from_dir);
    let normalized_to = normalize(to_path);
    let from_parts: Vec<&str> = normalized_from
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let to_parts: Vec<&str> = normalized_to
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();

    let mut common_len = 0usize;
    for (index, (from, to)) in from_parts.iter().zip(to_parts.iter()).enumerate() {
        if from == to {
            common_len = index + 1;
        } else {
            break;
        }
    }

    let mut result = Vec::new();
    for _ in 0..(from_parts.len() - common_len) {
        result.push("..");
    }

    for part in &to_parts[common_len..] {
        result.push(*part);
    }

    if result.is_empty() {
        to_parts.last().unwrap_or(&"").to_string()
    } else {
        result.join("/")
    }
}

pub fn to_standard_content_path(content_src: &str) -> String {
    if content_src.is_empty() {
        return String::new();
    }

    let path_part = normalize(content_src);
    if path_part.starts_with("OEBPS/") {
        return path_part;
    }

    if STANDARD_DIR_PREFIXES
        .iter()
        .any(|prefix| path_part.starts_with(prefix))
    {
        return format!("OEBPS/{}", path_part);
    }

    if is_text_document_href(&path_part) {
        return format!("OEBPS/Text/{}", path_part);
    }

    format!("OEBPS/{}", path_part)
}

pub fn is_text_like_file(path: &str) -> bool {
    let lower = normalize(path).to_lowercase();
    lower.ends_with(".xhtml")
        || lower.ends_with(".html")
        || lower.ends_with(".xml")
        || lower.ends_with(".css")
        || lower.ends_with(".svg")
}

pub fn is_text_document_href(href: &str) -> bool {
    let lower = normalize(href).to_lowercase();
    lower.ends_with(".xhtml") || lower.ends_with(".html")
}

fn normalize_path_part(path: &str) -> String {
    let decoded = percent_decode(path);
    let mut normalized = decoded
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();

    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_string();
    }

    normalized
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).unwrap_or_else(|_| value.to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_reference_suffix_preserves_anchor_and_query() {
        assert_eq!(
            split_reference_suffix("Text/chapter.xhtml?x=1#p2"),
            ("Text/chapter.xhtml", "?x=1#p2")
        );
        assert_eq!(
            normalize_reference(r#".\Text\chapter.xhtml#frag"#),
            "Text/chapter.xhtml#frag"
        );
        assert_eq!(
            normalize_reference("Text/Character%20Profile1.xhtml#frag"),
            "Text/Character Profile1.xhtml#frag"
        );
    }

    #[test]
    fn standard_path_preserves_nested_directories() {
        assert_eq!(
            standard_path("Text", "Text/part1/chapter.xhtml", &["Text"]),
            "OEBPS/Text/part1/chapter.xhtml"
        );
        assert_eq!(
            standard_path("Images", "images/covers/cover.jpg", &["Images", "Image"]),
            "OEBPS/Images/covers/cover.jpg"
        );
    }

    #[test]
    fn allocate_unique_paths_suffixes_collisions_case_insensitively() {
        let mut used = HashSet::new();

        assert_eq!(
            allocate_unique_path("OEBPS/Images/cover.jpg", &mut used),
            "OEBPS/Images/cover.jpg"
        );
        assert_eq!(
            allocate_unique_path("OEBPS/Images/COVER.jpg", &mut used),
            "OEBPS/Images/COVER-1.jpg"
        );
    }

    #[test]
    fn resolves_and_calculates_relative_paths() {
        assert_eq!(
            resolve_relative("OEBPS/Text", "../Images/cover.jpg"),
            "OEBPS/Images/cover.jpg"
        );
        assert_eq!(
            relative_path("OEBPS/Text", "OEBPS/Images/cover.jpg"),
            "../Images/cover.jpg"
        );
        assert_eq!(
            relative_path("OEBPS", "OEBPS/Text/chapter1.xhtml"),
            "Text/chapter1.xhtml"
        );
    }
}
