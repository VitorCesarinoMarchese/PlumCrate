use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

pub fn resolve_source_metadata(source_url: &str) -> SourceMetadata {
    let trimmed = source_url.trim();
    if !is_supported_source_url(trimmed) {
        return SourceMetadata::default();
    }

    let response = ureq::get(trimmed)
        .set("User-Agent", "PlumCrate/0.1 metadata resolver")
        .call();

    let Ok(response) = response else {
        return SourceMetadata::default();
    };
    if !(200..300).contains(&response.status()) {
        return SourceMetadata::default();
    }

    let Ok(html) = response.into_string() else {
        return SourceMetadata::default();
    };

    parse_source_metadata_html(&html)
}

fn is_supported_source_url(raw: &str) -> bool {
    raw.starts_with("https://www.curseforge.com/sims4/mods/")
        || raw.starts_with("https://curseforge.com/sims4/mods/")
        || raw.starts_with("https://modthesims.info/d/")
        || raw.starts_with("https://www.modthesims.info/d/")
        || raw.starts_with("https://modthesims.info/download.php")
        || raw.starts_with("https://www.modthesims.info/download.php")
}

pub fn parse_source_metadata_html(html: &str) -> SourceMetadata {
    SourceMetadata {
        display_name: meta_content(html, "og:title")
            .or_else(|| meta_content(html, "twitter:title"))
            .or_else(|| title_content(html)),
        preview_url: meta_content(html, "og:image")
            .or_else(|| meta_content(html, "twitter:image"))
            .or_else(|| meta_content(html, "twitter:image:src")),
    }
}

fn title_content(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let after_open = lower[start..].find('>')? + start + 1;
    let end = lower[after_open..].find("</title>")? + after_open;
    clean_html_value(&html[after_open..end])
}

fn meta_content(html: &str, key: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let mut offset = 0;
    while let Some(rel_start) = lower[offset..].find("<meta") {
        let start = offset + rel_start;
        let Some(rel_end) = lower[start..].find('>') else { break; };
        let end = start + rel_end + 1;
        let tag = &html[start..end];
        let tag_lower = &lower[start..end];
        let needle_property = format!("property=\"{}\"", key.to_lowercase());
        let needle_name = format!("name=\"{}\"", key.to_lowercase());
        let needle_property_single = format!("property='{}'", key.to_lowercase());
        let needle_name_single = format!("name='{}'", key.to_lowercase());
        if tag_lower.contains(&needle_property)
            || tag_lower.contains(&needle_name)
            || tag_lower.contains(&needle_property_single)
            || tag_lower.contains(&needle_name_single)
        {
            if let Some(value) = attr_value(tag, "content") {
                return clean_html_value(&value);
            }
        }
        offset = end;
    }
    None
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let pattern = format!("{}=", attr);
    let start = lower.find(&pattern)? + pattern.len();
    let rest = &tag[start..];
    let mut chars = rest.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = quote.len_utf8();
    let value_end = rest[value_start..].find(quote)? + value_start;
    Some(rest[value_start..value_end].to_string())
}

fn clean_html_value(value: &str) -> Option<String> {
    let cleaned = value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_graph_title_and_cover() {
        let metadata = parse_source_metadata_html(
            r#"<html><head>
                <meta property="og:title" content="MC Command Center" />
                <meta property="og:image" content="https://media.forgecdn.net/cover.jpg" />
            </head></html>"#,
        );

        assert_eq!(metadata.display_name.as_deref(), Some("MC Command Center"));
        assert_eq!(metadata.preview_url.as_deref(), Some("https://media.forgecdn.net/cover.jpg"));
    }

    #[test]
    fn falls_back_to_twitter_image_and_title_tag() {
        let metadata = parse_source_metadata_html(
            r#"<html><head>
                <title>Mod Title</title>
                <meta name='twitter:image' content='https://static.modthesims.info/preview.png'>
            </head></html>"#,
        );

        assert_eq!(metadata.display_name.as_deref(), Some("Mod Title"));
        assert_eq!(metadata.preview_url.as_deref(), Some("https://static.modthesims.info/preview.png"));
    }
}
