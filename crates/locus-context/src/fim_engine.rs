//! Fill-In-the-Middle (FIM) Context Engine for LOCUS.
//!
//! Handles multi-model FIM prompt template formatting (StarCoder, Qwen, Llama-3),
//! UTF-8 safe cursor context truncation, standard stop-token injection,
//! and monotonic request tracking.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FimTemplateFormat {
    /// `<｜fim begin｜>{prefix}<｜fim hole｜>{suffix}<｜fim end｜>`
    StarCoderDeepSeek,
    /// `<PRE> {prefix} <SUF> {suffix} <MID>`
    QwenCodeLlama,
    /// `<|fim_prefix|>{prefix}<|fim_suffix|>{suffix}<|fim_middle|>`
    Llama3Generic,
}

impl Default for FimTemplateFormat {
    fn default() -> Self {
        Self::QwenCodeLlama
    }
}

/// Request payload for inline code completion around the cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FimCompletionRequest {
    pub request_id: u64,
    pub file_path: String,
    pub prefix: String,
    pub suffix: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub max_tokens: usize,
    pub format: Option<FimTemplateFormat>,
}

/// Response payload containing the suggested code snippet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FimCompletionResponse {
    pub request_id: u64,
    pub suggested_text: String,
    pub latency_ms: u64,
    pub model_used: String,
    pub stop_reason: String,
}

/// Standard stop tokens for FIM generation across various models.
pub fn get_fim_stop_tokens(format: FimTemplateFormat) -> Vec<String> {
    match format {
        FimTemplateFormat::StarCoderDeepSeek => vec![
            "<｜fim end｜>".to_string(),
            "<｜fim begin｜>".to_string(),
            "<｜fim hole｜>".to_string(),
            "<|file_separator|>".to_string(),
            "\n\n\n".to_string(),
        ],
        FimTemplateFormat::QwenCodeLlama => vec![
            "<EOT>".to_string(),
            "<MID>".to_string(),
            "<PRE>".to_string(),
            "<SUF>".to_string(),
            "<file_sep>".to_string(),
            "\n\n\n".to_string(),
        ],
        FimTemplateFormat::Llama3Generic => vec![
            "<|fim_prefix|>".to_string(),
            "<|fim_suffix|>".to_string(),
            "<|fim_middle|>".to_string(),
            "<|end_of_text|>".to_string(),
            "\n\n\n".to_string(),
        ],
    }
}

/// Formats prefix and suffix into an exact FIM prompt according to the target model specification.
pub fn format_fim_prompt(prefix: &str, suffix: &str, format: FimTemplateFormat) -> String {
    match format {
        FimTemplateFormat::StarCoderDeepSeek => {
            format!("<｜fim begin｜>{}<｜fim hole｜>{}<｜fim end｜>", prefix, suffix)
        }
        FimTemplateFormat::QwenCodeLlama => {
            format!("<PRE> {} <SUF> {} <MID>", prefix, suffix)
        }
        FimTemplateFormat::Llama3Generic => {
            format!("<|fim_prefix|>{}<|fim_suffix|>{}<|fim_middle|>", prefix, suffix)
        }
    }
}

/// UTF-8 safe line-based cursor context slicing.
///
/// Takes a bounded window of lines around the cursor (default: 60 prefix lines, 30 suffix lines)
/// and strictly guards against mid-character byte slicing panics on Arabic, Unicode, or multi-byte text.
pub fn truncate_cursor_context(
    content: &str,
    cursor_line: usize, // 1-indexed
    cursor_col: usize,  // 1-indexed
    max_prefix_lines: usize,
    max_suffix_lines: usize,
) -> (String, String) {
    if content.is_empty() {
        return (String::new(), String::new());
    }

    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    if total_lines == 0 {
        return (String::new(), String::new());
    }

    let line_idx = if cursor_line > 0 && cursor_line <= total_lines {
        cursor_line - 1
    } else {
        total_lines - 1
    };

    // 1. Gather prefix lines (up to max_prefix_lines prior to cursor line)
    let start_line = line_idx.saturating_sub(max_prefix_lines);
    let mut prefix_parts: Vec<&str> = Vec::new();
    for i in start_line..line_idx {
        prefix_parts.push(all_lines[i]);
    }

    // Handle cursor line's prefix part (char-safe UTF-8 slicing)
    let current_line = all_lines[line_idx];
    let char_count = current_line.chars().count();
    let col_char_limit = if cursor_col > 0 && cursor_col <= char_count + 1 {
        cursor_col - 1
    } else {
        char_count
    };

    let line_prefix: String = current_line.chars().take(col_char_limit).collect();
    let line_suffix: String = current_line.chars().skip(col_char_limit).collect();

    let prefix = if prefix_parts.is_empty() {
        line_prefix
    } else {
        let mut p = prefix_parts.join("\n");
        p.push('\n');
        p.push_str(&line_prefix);
        p
    };

    // 2. Gather suffix lines (up to max_suffix_lines after cursor line)
    let mut suffix_parts: Vec<&str> = Vec::new();
    let end_line = (line_idx + 1 + max_suffix_lines).min(total_lines);
    for i in (line_idx + 1)..end_line {
        suffix_parts.push(all_lines[i]);
    }

    let suffix = if suffix_parts.is_empty() {
        line_suffix
    } else if line_suffix.is_empty() {
        suffix_parts.join("\n")
    } else {
        let mut s = line_suffix;
        s.push('\n');
        s.push_str(&suffix_parts.join("\n"));
        s
    };

    (prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_fim_prompts() {
        let prefix = "def calculate_sum(a, b):\n";
        let suffix = "\n    return result";

        let prompt_qwen = format_fim_prompt(prefix, suffix, FimTemplateFormat::QwenCodeLlama);
        assert!(prompt_qwen.contains("<PRE>"));
        assert!(prompt_qwen.contains("<SUF>"));
        assert!(prompt_qwen.contains("<MID>"));

        let prompt_starcoder = format_fim_prompt(prefix, suffix, FimTemplateFormat::StarCoderDeepSeek);
        assert!(prompt_starcoder.contains("<｜fim begin｜>"));
        assert!(prompt_starcoder.contains("<｜fim hole｜>"));
        assert!(prompt_starcoder.contains("<｜fim end｜>"));

        let prompt_llama = format_fim_prompt(prefix, suffix, FimTemplateFormat::Llama3Generic);
        assert!(prompt_llama.contains("<|fim_prefix|>"));
        assert!(prompt_llama.contains("<|fim_suffix|>"));
        assert!(prompt_llama.contains("<|fim_middle|>"));
    }

    #[test]
    fn test_utf8_safe_cursor_slicing_arabic_and_symbols() {
        // Multi-byte Unicode content with Arabic comments
        let code = "fn main() {\n    // مرحبا بالعالم: اختبار الأمان\n    let x = 42;\n    println!(\"{}\", x);\n}";
        // Cursor on line 2, character column 15 (inside Arabic text)
        let (prefix, suffix) = truncate_cursor_context(code, 2, 15, 60, 30);

        assert!(!prefix.is_empty());
        assert!(!suffix.is_empty());
        assert!(prefix.contains("fn main()"));
        assert!(suffix.contains("let x = 42;"));
    }

    #[test]
    fn test_cursor_slicing_empty_and_bounds() {
        let (p, s) = truncate_cursor_context("", 1, 1, 60, 30);
        assert_eq!(p, "");
        assert_eq!(s, "");

        let one_line = "let a = 10;";
        let (p1, s1) = truncate_cursor_context(one_line, 1, 5, 60, 30);
        assert_eq!(p1, "let ");
        assert_eq!(s1, "a = 10;");
    }

    #[test]
    fn test_stop_tokens_presence() {
        let stops = get_fim_stop_tokens(FimTemplateFormat::StarCoderDeepSeek);
        assert!(stops.contains(&"<｜fim end｜>".to_string()));
        assert!(stops.contains(&"<|file_separator|>".to_string()));
    }
}
