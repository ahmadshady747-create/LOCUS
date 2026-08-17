use serde::{Deserialize, Serialize};

/// Statistics tracking token reduction achieved by code skeletonization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkeletonStats {
    pub original_chars: usize,
    pub skeleton_chars: usize,
    pub original_tokens_est: usize,
    pub skeleton_tokens_est: usize,
    pub tokens_saved_est: usize,
    pub reduction_percentage: f64,
}

/// Extracts structural code skeletons (signatures, structs, traits, interfaces, docstrings)
/// while stripping function and method bodies to save context tokens.
/// Safely ignores braces inside strings and comments.
pub fn extract_skeleton(code: &str, extension: &str) -> String {
    let clean_ext = extension.trim().trim_start_matches('.').to_lowercase();

    match clean_ext.as_str() {
        "rs" => skeletonize_rust(code),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => skeletonize_js_ts(code),
        "py" | "pyw" => skeletonize_python(code),
        "go" => skeletonize_c_like(code),
        "c" | "cpp" | "h" | "hpp" | "cs" | "java" => skeletonize_c_like(code),
        _ => skeletonize_generic(code),
    }
}

/// Calculates approximate token reduction metrics
pub fn calculate_skeleton_savings(original: &str, skeleton: &str) -> SkeletonStats {
    let original_chars = original.len();
    let skeleton_chars = skeleton.len();
    let original_tokens_est = (original_chars + 3) / 4;
    let skeleton_tokens_est = (skeleton_chars + 3) / 4;
    let tokens_saved_est = original_tokens_est.saturating_sub(skeleton_tokens_est);
    let reduction_percentage = if original_tokens_est > 0 {
        (tokens_saved_est as f64 / original_tokens_est as f64) * 100.0
    } else {
        0.0
    };

    SkeletonStats {
        original_chars,
        skeleton_chars,
        original_tokens_est,
        skeleton_tokens_est,
        tokens_saved_est,
        reduction_percentage,
    }
}

// -----------------------------------------------------------------------------
// Rust Skeletonizer
// -----------------------------------------------------------------------------
fn skeletonize_rust(code: &str) -> String {
    let mut out = String::with_capacity(code.len() / 2);
    let lines: Vec<&str> = code.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Preserve comments, attributes, imports, structs, enums, traits, types, and consts
        if trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("//")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("pub mod ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("pub type ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("static ")
            || trimmed.starts_with("pub static ")
        {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }

        // Check for function or method declaration
        if is_rust_fn_header(trimmed) {
            let mut fn_sig = line.to_string();

            // Handle multi-line function signatures up until opening brace '{' or semicolon ';'
            while !fn_sig.contains('{') && !fn_sig.contains(';') && i + 1 < lines.len() {
                i += 1;
                fn_sig.push('\n');
                fn_sig.push_str(lines[i]);
            }

            if fn_sig.contains('{') {
                // Strip function body
                let before_brace = fn_sig.rfind('{').map(|pos| &fn_sig[..pos]).unwrap_or(&fn_sig).trim_end();
                out.push_str(before_brace);
                out.push_str(" { /* ... */ }\n");

                // Skip the function body safely tracking string/comment-aware braces
                i = skip_brace_block_rust(&lines, i);
            } else {
                out.push_str(&fn_sig);
                out.push('\n');
                i += 1;
            }
            continue;
        }

        // Default: keep line
        out.push_str(line);
        out.push('\n');
        i += 1;
    }

    out
}

fn is_rust_fn_header(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("pub(crate) fn ")
        || trimmed.starts_with("pub(super) fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("pub(crate) async fn ")
        || trimmed.starts_with("const fn ")
        || trimmed.starts_with("pub const fn ")
        || trimmed.starts_with("unsafe fn ")
        || trimmed.starts_with("pub unsafe fn ")
}

// -----------------------------------------------------------------------------
// TypeScript / JavaScript Skeletonizer
// -----------------------------------------------------------------------------
fn skeletonize_js_ts(code: &str) -> String {
    let mut out = String::with_capacity(code.len() / 2);
    let lines: Vec<&str> = code.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Preserve comments, imports, interfaces, types, exports
        if trimmed.starts_with("/**")
            || trimmed.starts_with("*")
            || trimmed.starts_with("*/")
            || trimmed.starts_with("//")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("export interface ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("export type ")
            || trimmed.starts_with("type ")
        {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }

        // Function / Method detection
        if is_js_fn_header(trimmed) {
            let mut fn_sig = line.to_string();

            while !fn_sig.contains('{') && !fn_sig.contains(';') && i + 1 < lines.len() {
                i += 1;
                fn_sig.push('\n');
                fn_sig.push_str(lines[i]);
            }

            if fn_sig.contains('{') {
                let before_brace = fn_sig.rfind('{').map(|pos| &fn_sig[..pos]).unwrap_or(&fn_sig).trim_end();
                out.push_str(before_brace);
                out.push_str(" { /* ... */ }\n");
                i = skip_brace_block_rust(&lines, i);
            } else {
                out.push_str(&fn_sig);
                out.push('\n');
                i += 1;
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
        i += 1;
    }

    out
}

fn is_js_fn_header(trimmed: &str) -> bool {
    trimmed.starts_with("function ")
        || trimmed.starts_with("export function ")
        || trimmed.starts_with("export async function ")
        || trimmed.starts_with("async function ")
        || (trimmed.contains('(') && trimmed.contains(')') && trimmed.ends_with('{') && !trimmed.starts_with("if") && !trimmed.starts_with("for") && !trimmed.starts_with("while") && !trimmed.starts_with("switch"))
}

// -----------------------------------------------------------------------------
// Python Skeletonizer
// -----------------------------------------------------------------------------
fn skeletonize_python(code: &str) -> String {
    let mut out = String::with_capacity(code.len() / 2);
    let lines: Vec<&str> = code.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Preserve comments, imports, class definitions, decorators
        if trimmed.starts_with('#')
            || trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with('@')
            || trimmed.starts_with("class ")
        {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }

        // Function / Method detection in Python
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            let indent = get_indentation(line);
            let mut fn_sig = line.to_string();

            while !fn_sig.contains(':') && i + 1 < lines.len() {
                i += 1;
                fn_sig.push('\n');
                fn_sig.push_str(lines[i]);
            }

            out.push_str(&fn_sig);
            out.push('\n');

            let base_indent_len = indent.len();
            i += 1;

            // Check if there is an immediate docstring
            let mut has_docstring = false;
            if i < lines.len() {
                let next_trimmed = lines[i].trim();
                if next_trimmed.starts_with("\"\"\"") || next_trimmed.starts_with("'''") {
                    has_docstring = true;
                    out.push_str(lines[i]);
                    out.push('\n');

                    // If single line docstring
                    let is_closed = next_trimmed.len() > 3 && (next_trimmed[3..].contains("\"\"\"") || next_trimmed[3..].contains("'''"));
                    if !is_closed {
                        i += 1;
                        while i < lines.len() {
                            out.push_str(lines[i]);
                            out.push('\n');
                            if lines[i].contains("\"\"\"") || lines[i].contains("'''") {
                                break;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                }
            }

            // Skip remaining function body lines that are deeper indented
            while i < lines.len() {
                let body_line = lines[i];
                let body_trimmed = body_line.trim();

                if body_trimmed.is_empty() {
                    i += 1;
                    continue;
                }

                let line_indent_len = get_indentation(body_line).len();
                if line_indent_len > base_indent_len {
                    i += 1; // skip body line
                } else {
                    break; // Reached end of function body
                }
            }

            if !has_docstring {
                out.push_str(&indent);
                out.push_str("    ...\n");
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
        i += 1;
    }

    out
}

// -----------------------------------------------------------------------------
// Generic C-like / Fallback Skeletonizer
// -----------------------------------------------------------------------------
fn skeletonize_c_like(code: &str) -> String {
    skeletonize_rust(code)
}

fn skeletonize_generic(code: &str) -> String {
    code.to_string()
}

// -----------------------------------------------------------------------------
// Helper: Safe Brace Counting Skipping Strings & Comments
// -----------------------------------------------------------------------------
fn skip_brace_block_rust(lines: &[&str], start_line_idx: usize) -> usize {
    let mut brace_depth = 0;
    let mut started = false;

    let mut in_block_comment = false;
    let mut in_string = false;
    let mut in_char = false;
    let mut in_raw_string = false;

    let mut i = start_line_idx;

    while i < lines.len() {
        let line = lines[i];
        let chars: Vec<char> = line.chars().collect();
        let mut c_idx = 0;

        // On the signature line, start right at the body opening brace to ignore parameter destructuring braces
        if i == start_line_idx {
            if let Some(last_open_brace) = line.rfind('{') {
                c_idx = last_open_brace;
            }
        }

        while c_idx < chars.len() {
            let c = chars[c_idx];
            let next_c = chars.get(c_idx + 1).copied();

            if in_block_comment {
                if c == '*' && next_c == Some('/') {
                    in_block_comment = false;
                    c_idx += 2;
                    continue;
                }
                c_idx += 1;
                continue;
            }

            if in_string {
                if c == '\\' {
                    c_idx += 2; // skip escaped char
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                c_idx += 1;
                continue;
            }

            if in_char {
                if c == '\\' {
                    c_idx += 2;
                    continue;
                }
                if c == '\'' {
                    in_char = false;
                }
                c_idx += 1;
                continue;
            }

            if in_raw_string {
                if c == '"' && next_c == Some('#') {
                    in_raw_string = false;
                    c_idx += 2;
                    continue;
                }
                c_idx += 1;
                continue;
            }

            // Normal code tokens
            if c == '/' && next_c == Some('/') {
                break;
            }
            if c == '/' && next_c == Some('*') {
                in_block_comment = true;
                c_idx += 2;
                continue;
            }
            if c == 'r' && next_c == Some('#') {
                in_raw_string = true;
                c_idx += 2;
                continue;
            }
            if c == '"' {
                in_string = true;
                c_idx += 1;
                continue;
            }
            if c == '\'' {
                in_char = true;
                c_idx += 1;
                continue;
            }

            // Brace counting
            if c == '{' {
                brace_depth += 1;
                started = true;
            } else if c == '}' {
                brace_depth -= 1;
                if started && brace_depth <= 0 {
                    return i + 1; // Completed the outer block
                }
            }

            c_idx += 1;
        }

        if started && brace_depth <= 0 {
            return i + 1;
        }

        i += 1;
    }

    i
}

fn get_indentation(line: &str) -> String {
    let spaces: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    spaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_skeleton_rust_safe_braces() {
        let rust_code = r##"
pub struct UserConfig {
    pub name: String,
    pub timeout: u64,
}

/// Calculate something important
pub async fn process_data(id: u32, config: &UserConfig) -> Result<String> {
    let tricky_str = "braces inside string { } { { }}";
    let comment_with_braces = 123; // { } comment
    let raw_str = r#"even more { braces } here"#;
    println!("Hello {}", tricky_str);
    Ok(format!("done: {}", id))
}

pub trait DataHandler {
    fn handle(&self);
}
"##;

        let skeleton = extract_skeleton(rust_code, "rs");
        assert!(skeleton.contains("pub struct UserConfig"));
        assert!(skeleton.contains("pub trait DataHandler"));
        assert!(skeleton.contains("pub async fn process_data(id: u32, config: &UserConfig) -> Result<String> { /* ... */ }"));
        // Function body should be stripped
        assert!(!skeleton.contains("let tricky_str ="));
        assert!(!skeleton.contains("println!"));

        let savings = calculate_skeleton_savings(rust_code, &skeleton);
        assert!(savings.reduction_percentage > 30.0);
    }

    #[test]
    fn test_extract_skeleton_python() {
        let py_code = r##"
import os
import sys

class ModelProcessor:
    """Manages model lifecycle and inference."""
    
    def __init__(self, model_name: str, temperature: float = 0.7):
        """Initializes the processor."""
        self.model_name = model_name
        self.temperature = temperature
        self.client = None
        self._setup()

    def process(self, query: str) -> dict:
        result = {}
        for step in range(10):
            result[step] = query.upper()
        return result
"##;

        let skeleton = extract_skeleton(py_code, "py");
        assert!(skeleton.contains("class ModelProcessor:"));
        assert!(skeleton.contains("def __init__(self, model_name: str, temperature: float = 0.7):"));
        assert!(skeleton.contains("def process(self, query: str) -> dict:"));
        // Inner loops stripped
        assert!(!skeleton.contains("result[step] = query.upper()"));
    }

    #[test]
    fn test_extract_skeleton_typescript() {
        let ts_code = r##"
import React from "react";

export interface AppProps {
    title: string;
    count: number;
}

export function CounterComponent({ title, count }: AppProps) {
    const [val, setVal] = React.useState(count);
    const complexObj = { key: "{ braces in value }" };
    return <div>{title}: {val}</div>;
}
"##;

        let skeleton = extract_skeleton(ts_code, "tsx");
        assert!(skeleton.contains("export interface AppProps"));
        assert!(skeleton.contains("export function CounterComponent({ title, count }: AppProps) { /* ... */ }"));
        assert!(!skeleton.contains("const [val, setVal]"));
    }
}
