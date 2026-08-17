//! Semantic HTML-to-Markdown content extractor & local documentation cache.

use crate::types::{DocQuery, DocSearchResult};
use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

pub struct SemanticExtractor;

impl SemanticExtractor {
    /// Strips boilerplate HTML tags and transforms meaningful content into high-density Markdown.
    pub fn html_to_dense_markdown(html: &str) -> String {
        let mut text = html.to_string();

        // 1. Strip comments
        let re_comments = Regex::new(r"(?s)<!--.*?-->").unwrap();
        text = re_comments.replace_all(&text, "").to_string();

        // 2. Strip noise container elements entirely (nav, header, footer, script, style, aside, iframe, noscript, svg)
        let noise_tags = [
            "script", "style", "nav", "header", "footer", "aside", "iframe", "noscript", "svg",
            "select", "button", "form",
        ];
        for tag in noise_tags {
            let re_tag = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}>", tag = tag)).unwrap();
            text = re_tag.replace_all(&text, "").to_string();
        }

        // 3. Convert code blocks (<pre><code>...</code></pre>)
        let re_pre_code = Regex::new(r#"(?is)<pre\b[^>]*><code(?:\s+class="[^"]*language-([^"\s]+)[^"]*")?[^>]*>(.*?)</code></pre>"#).unwrap();
        text = re_pre_code
            .replace_all(&text, |caps: &regex::Captures| {
                let lang = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let code = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let clean_code = Self::decode_html_entities(code);
                format!("\n```{}\n{}\n```\n", lang, clean_code.trim())
            })
            .to_string();

        // 4. Convert inline code (<code>...</code>)
        let re_inline_code = Regex::new(r"(?is)<code\b[^>]*>(.*?)</code>").unwrap();
        text = re_inline_code
            .replace_all(&text, |caps: &regex::Captures| {
                let code = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let clean = Self::decode_html_entities(code).trim().to_string();
                format!("`{}`", clean)
            })
            .to_string();

        // 5. Convert Headings (h1..h6)
        for level in 1..=6 {
            let hashes = "#".repeat(level);
            let re_h = Regex::new(&format!(r"(?is)<h{}\b[^>]*>(.*?)</h{}>", level, level)).unwrap();
            text = re_h
                .replace_all(&text, |caps: &regex::Captures| {
                    let title = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let clean = Self::strip_tags(title);
                    format!("\n{} {}\n", hashes, clean.trim())
                })
                .to_string();
        }

        // 6. Convert list items
        let re_li = Regex::new(r"(?is)<li\b[^>]*>(.*?)</li>").unwrap();
        text = re_li
            .replace_all(&text, |caps: &regex::Captures| {
                let item = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let clean = Self::strip_tags(item);
                format!("\n- {}\n", clean.trim())
            })
            .to_string();

        // 7. Convert paragraphs & breaks
        let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
        text = re_br.replace_all(&text, "\n").to_string();

        let re_p = Regex::new(r"(?is)<p\b[^>]*>(.*?)</p>").unwrap();
        text = re_p
            .replace_all(&text, |caps: &regex::Captures| {
                let p = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let clean = Self::strip_tags(p);
                format!("\n{}\n", clean.trim())
            })
            .to_string();

        // 8. Strip all remaining HTML tags
        text = Self::strip_tags(&text);

        // 9. Decode HTML entities
        text = Self::decode_html_entities(&text);

        // 10. Clean up extra blank lines and normalize whitespace
        let re_multi_newlines = Regex::new(r"\n{3,}").unwrap();
        text = re_multi_newlines.replace_all(&text, "\n\n").to_string();

        text.trim().to_string()
    }

    fn strip_tags(html: &str) -> String {
        let re_tags = Regex::new(r"<[^>]*>").unwrap();
        re_tags.replace_all(html, "").to_string()
    }

    fn decode_html_entities(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
    }
}

// ---------------------------------------------------------------------------
// Local Documentation Cache Layer
// ---------------------------------------------------------------------------

pub struct DocsCacheManager;

impl DocsCacheManager {
    fn cache_dir() -> PathBuf {
        let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join(".locus").join("cache").join("docs")
    }

    fn cache_key(query: &DocQuery) -> String {
        let raw = format!(
            "{}:{}:{}",
            query.ecosystem.display_name(),
            query.query.to_lowercase().trim(),
            query.version.as_deref().unwrap_or("latest")
        );
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&raw, &mut hasher);
        format!("{:016x}", std::hash::Hasher::finish(&hasher))
    }

    pub fn get_cached(query: &DocQuery) -> Option<DocSearchResult> {
        let dir = Self::cache_dir();
        let key = Self::cache_key(query);
        let file_path = dir.join(format!("{}.json", key));

        if !file_path.exists() {
            return None;
        }

        // Check 7-day TTL
        if let Ok(meta) = fs::metadata(&file_path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = SystemTime::now().duration_since(modified) {
                    if age > Duration::from_secs(7 * 24 * 3600) {
                        debug!("Docs cache expired for key {}", key);
                        let _ = fs::remove_file(&file_path);
                        return None;
                    }
                }
            }
        }

        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(mut res) = serde_json::from_str::<DocSearchResult>(&content) {
                res.cached = true;
                info!("Docs cache HIT for query '{}'", query.query);
                return Some(res);
            }
        }

        None
    }

    pub fn store_cache(query: &DocQuery, result: &DocSearchResult) -> Result<()> {
        let dir = Self::cache_dir();
        fs::create_dir_all(&dir)?;

        let key = Self::cache_key(query);
        let file_path = dir.join(format!("{}.json", key));

        let json = serde_json::to_string_pretty(result)?;
        fs::write(file_path, json)?;
        debug!("Cached docs for query '{}'", query.query);
        Ok(())
    }

    pub fn clear_cache() -> Result<u32> {
        let dir = Self::cache_dir();
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    let _ = fs::remove_file(entry.path());
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_dense_markdown_strips_noise() {
        let html = r#"
            <html>
                <head><style>.btn { color: red; }</style></head>
                <body>
                    <nav><a href="/home">Home</a></nav>
                    <h1>My Package</h1>
                    <p>This is a <b>fast</b> library.</p>
                    <pre><code class="language-rust">fn main() { println!("hi"); }</code></pre>
                    <footer>Copyright 2026</footer>
                </body>
            </html>
        "#;

        let md = SemanticExtractor::html_to_dense_markdown(html);
        assert!(md.contains("# My Package"));
        assert!(md.contains("This is a fast library."));
        assert!(md.contains("```rust\nfn main() { println!(\"hi\"); }\n```"));
        assert!(!md.contains("Copyright 2026"));
        assert!(!md.contains(".btn { color: red; }"));
        assert!(!md.contains("Home"));
    }
}
