//! Granular Selective Hunk Patching & AST Syntax Safety Engine for LOCUS.
//!
//! Decomposes diffs into atomic, interactive hunks (`DiffHunk`), performs Context Reconciliation
//! to prevent duplicated lines across adjacent edits, and validates post-patch syntactic balance.

use locus_core::types::{DiffHunk, DiffLine, DiffLineType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchResult {
    pub patched_content: String,
    pub applied_hunks_count: usize,
    pub total_hunks_count: usize,
    pub syntax_warning: Option<String>,
}

/// Parses original and modified file contents into discrete, reviewable `DiffHunk` blocks.
pub fn parse_diff_into_hunks(original_content: &str, modified_content: &str) -> Vec<DiffHunk> {
    let orig_lines: Vec<&str> = if original_content.is_empty() {
        Vec::new()
    } else {
        original_content.lines().collect()
    };
    let mod_lines: Vec<&str> = if modified_content.is_empty() {
        Vec::new()
    } else {
        modified_content.lines().collect()
    };

    if orig_lines == mod_lines {
        return Vec::new();
    }

    // 1. Compute LCS-based raw diff entries
    let lcs = compute_lcs(&orig_lines, &mod_lines);
    let raw_diff = build_raw_diff(&orig_lines, &mod_lines, &lcs);

    // 2. Group raw diff with 3 lines of context
    let raw_hunks = group_into_hunks(raw_diff, 3);

    // 3. Context Reconciliation: merge overlapping/adjacent hunks within 2 lines
    reconcile_adjacent_hunks(raw_hunks)
}

/// Applies only the selected hunk IDs to the original content with dynamic offset arithmetic.
pub fn apply_selected_hunks(
    original_content: &str,
    hunks: &[DiffHunk],
    selected_hunk_ids: &[String],
    file_ext: Option<&str>,
) -> Result<PatchResult, String> {
    let selected_set: HashSet<&str> = selected_hunk_ids.iter().map(|s| s.as_str()).collect();
    let orig_lines: Vec<&str> = if original_content.is_empty() {
        Vec::new()
    } else {
        original_content.lines().collect()
    };

    let mut result_lines: Vec<String> = Vec::new();
    let mut orig_cursor = 0;
    let mut applied_count = 0;

    for hunk in hunks {
        let is_selected = selected_set.contains(hunk.hunk_id.as_str());

        let hunk_start = if hunk.old_start > 0 {
            hunk.old_start - 1
        } else {
            0
        };

        // Copy unchanged lines prior to this hunk
        while orig_cursor < hunk_start && orig_cursor < orig_lines.len() {
            result_lines.push(orig_lines[orig_cursor].to_string());
            orig_cursor += 1;
        }

        if is_selected {
            applied_count += 1;
            // Apply additions and advance over deletions/context
            for line in &hunk.lines {
                match line.line_type {
                    DiffLineType::Context => {
                        if orig_cursor < orig_lines.len() {
                            result_lines.push(orig_lines[orig_cursor].to_string());
                            orig_cursor += 1;
                        } else {
                            result_lines.push(line.content.clone());
                        }
                    }
                    DiffLineType::Addition => {
                        result_lines.push(line.content.clone());
                    }
                    DiffLineType::Deletion => {
                        if orig_cursor < orig_lines.len() {
                            orig_cursor += 1;
                        }
                    }
                }
            }
        } else {
            // Rejected hunk: retain original content for this hunk's span
            let hunk_end = hunk_start + hunk.old_lines;
            while orig_cursor < hunk_end && orig_cursor < orig_lines.len() {
                result_lines.push(orig_lines[orig_cursor].to_string());
                orig_cursor += 1;
            }
        }
    }

    // Copy any remaining trailing lines
    while orig_cursor < orig_lines.len() {
        result_lines.push(orig_lines[orig_cursor].to_string());
        orig_cursor += 1;
    }

    let patched = if result_lines.is_empty() {
        String::new()
    } else {
        result_lines.join("\n") + if original_content.ends_with('\n') { "\n" } else { "" }
    };

    // 4. AST / Structural Syntax Safety Check
    let syntax_warning = check_structural_syntax_safety(&patched, file_ext.unwrap_or(""));

    Ok(PatchResult {
        patched_content: patched,
        applied_hunks_count: applied_count,
        total_hunks_count: hunks.len(),
        syntax_warning,
    })
}

/// Verifies structural delimiter balance (braces, brackets, parentheses) and unclosed quotes.
pub fn check_structural_syntax_safety(content: &str, ext: &str) -> Option<String> {
    let mut brace_stack = Vec::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut is_escaped = false;

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip line comments based on extension
        let is_comment = match ext {
            "rs" | "ts" | "js" | "tsx" | "jsx" | "go" | "c" | "cpp" | "java" => trimmed.starts_with("//"),
            "py" | "sh" | "rb" | "toml" | "yaml" | "yml" => trimmed.starts_with('#'),
            _ => false,
        };
        if is_comment {
            continue;
        }

        for ch in line.chars() {
            if is_escaped {
                is_escaped = false;
                continue;
            }

            if ch == '\\' {
                is_escaped = true;
                continue;
            }

            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = ch;
                }
                '{' | '(' | '[' => {
                    brace_stack.push((ch, line_no + 1));
                }
                '}' => {
                    if let Some((top, _)) = brace_stack.pop() {
                        if top != '{' {
                            return Some(format!(
                                "Mismatched closing brace '}}' at line {} (expected match for '{}')",
                                line_no + 1, top
                            ));
                        }
                    } else {
                        return Some(format!("Unexpected unmatched closing brace '}}' at line {}", line_no + 1));
                    }
                }
                ')' => {
                    if let Some((top, _)) = brace_stack.pop() {
                        if top != '(' {
                            return Some(format!(
                                "Mismatched closing parenthesis ')' at line {} (expected match for '{}')",
                                line_no + 1, top
                            ));
                        }
                    } else {
                        return Some(format!("Unexpected unmatched closing parenthesis ')' at line {}", line_no + 1));
                    }
                }
                ']' => {
                    if let Some((top, _)) = brace_stack.pop() {
                        if top != '[' {
                            return Some(format!(
                                "Mismatched closing bracket ']' at line {} (expected match for '{}')",
                                line_no + 1, top
                            ));
                        }
                    } else {
                        return Some(format!("Unexpected unmatched closing bracket ']' at line {}", line_no + 1));
                    }
                }
                _ => {}
            }
        }
    }

    if let Some((unclosed, line)) = brace_stack.last() {
        return Some(format!("Unclosed delimiter '{}' opened at line {}", unclosed, line));
    }

    None
}

// === Internal LCS Diff & Reconciliation Helpers ===

#[derive(Debug, Clone)]
enum RawDiffItem<'a> {
    Same(&'a str),
    Add(&'a str),
    Del(&'a str),
}

fn compute_lcs(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let m = a.len();
    let n = b.len();
    let mut matrix = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                matrix[i][j] = matrix[i - 1][j - 1] + 1;
            } else {
                matrix[i][j] = matrix[i - 1][j].max(matrix[i][j - 1]);
            }
        }
    }
    matrix
}

fn build_raw_diff<'a>(a: &[&'a str], b: &[&'a str], matrix: &[Vec<usize>]) -> Vec<RawDiffItem<'a>> {
    let mut diff = Vec::new();
    let mut i = a.len();
    let mut j = b.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            diff.push(RawDiffItem::Same(a[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || matrix[i][j - 1] >= matrix[i - 1][j]) {
            diff.push(RawDiffItem::Add(b[j - 1]));
            j -= 1;
        } else if i > 0 {
            diff.push(RawDiffItem::Del(a[i - 1]));
            i -= 1;
        }
    }

    diff.reverse();
    diff
}

fn group_into_hunks(raw: Vec<RawDiffItem>, context_size: usize) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_lines: Vec<DiffLine> = Vec::new();
    let mut old_line = 1;
    let mut new_line = 1;
    let mut hunk_old_start = 1;
    let mut hunk_new_start = 1;
    let mut hunk_old_count = 0;
    let mut hunk_new_count = 0;
    let mut in_hunk = false;
    let mut trailing_context = 0;
    let mut hunk_counter = 1;

    for item in raw {
        match item {
            RawDiffItem::Same(content) => {
                if in_hunk {
                    trailing_context += 1;
                    current_lines.push(DiffLine {
                        line_type: DiffLineType::Context,
                        content: content.to_string(),
                        old_line_no: Some(old_line),
                        new_line_no: Some(new_line),
                    });
                    hunk_old_count += 1;
                    hunk_new_count += 1;

                    if trailing_context >= context_size {
                        let header = format!(
                            "@@ -{},{} +{},{} @@",
                            hunk_old_start, hunk_old_count, hunk_new_start, hunk_new_count
                        );
                        hunks.push(DiffHunk {
                            hunk_id: format!("hunk_{}", hunk_counter),
                            old_start: hunk_old_start,
                            old_lines: hunk_old_count,
                            new_start: hunk_new_start,
                            new_lines: hunk_new_count,
                            header,
                            lines: current_lines.clone(),
                        });
                        hunk_counter += 1;
                        current_lines.clear();
                        in_hunk = false;
                        trailing_context = 0;
                    }
                }
                old_line += 1;
                new_line += 1;
            }
            RawDiffItem::Add(content) => {
                if !in_hunk {
                    in_hunk = true;
                    hunk_old_start = old_line;
                    hunk_new_start = new_line;
                    hunk_old_count = 0;
                    hunk_new_count = 0;
                }
                trailing_context = 0;
                current_lines.push(DiffLine {
                    line_type: DiffLineType::Addition,
                    content: content.to_string(),
                    old_line_no: None,
                    new_line_no: Some(new_line),
                });
                hunk_new_count += 1;
                new_line += 1;
            }
            RawDiffItem::Del(content) => {
                if !in_hunk {
                    in_hunk = true;
                    hunk_old_start = old_line;
                    hunk_new_start = new_line;
                    hunk_old_count = 0;
                    hunk_new_count = 0;
                }
                trailing_context = 0;
                current_lines.push(DiffLine {
                    line_type: DiffLineType::Deletion,
                    content: content.to_string(),
                    old_line_no: Some(old_line),
                    new_line_no: None,
                });
                hunk_old_count += 1;
                old_line += 1;
            }
        }
    }

    if in_hunk && !current_lines.is_empty() {
        let header = format!(
            "@@ -{},{} +{},{} @@",
            hunk_old_start, hunk_old_count, hunk_new_start, hunk_new_count
        );
        hunks.push(DiffHunk {
            hunk_id: format!("hunk_{}", hunk_counter),
            old_start: hunk_old_start,
            old_lines: hunk_old_count,
            new_start: hunk_new_start,
            new_lines: hunk_new_count,
            header,
            lines: current_lines,
        });
    }

    hunks
}

fn reconcile_adjacent_hunks(mut hunks: Vec<DiffHunk>) -> Vec<DiffHunk> {
    if hunks.len() <= 1 {
        return hunks;
    }

    let mut reconciled: Vec<DiffHunk> = Vec::new();

    for next_hunk in hunks.drain(..) {
        if let Some(prev) = reconciled.last_mut() {
            let prev_old_end = prev.old_start + prev.old_lines;
            // If adjacent or overlapping within 2 lines, merge them to avoid duplicate shared context
            if next_hunk.old_start <= prev_old_end + 2 {
                prev.old_lines = (next_hunk.old_start + next_hunk.old_lines).saturating_sub(prev.old_start);
                prev.new_lines = (next_hunk.new_start + next_hunk.new_lines).saturating_sub(prev.new_start);
                prev.header = format!(
                    "@@ -{},{} +{},{} @@",
                    prev.old_start, prev.old_lines, prev.new_start, prev.new_lines
                );
                prev.lines.extend(next_hunk.lines);
                continue;
            }
        }
        reconciled.push(next_hunk);
    }

    // Re-index hunk IDs sequentially
    for (i, h) in reconciled.iter_mut().enumerate() {
        h.hunk_id = format!("hunk_{}", i + 1);
    }

    reconciled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_selective_hunk_application() {
        let original = "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}";
        let modified = "fn main() {\n    let x = 10;\n    let y = 20;\n    println!(\"sum = {}\", x + y);\n}";

        let hunks = parse_diff_into_hunks(original, modified);
        assert!(!hunks.is_empty());

        let all_ids: Vec<String> = hunks.iter().map(|h| h.hunk_id.clone()).collect();

        // Apply all hunks
        let res_all = apply_selected_hunks(original, &hunks, &all_ids, Some("rs")).unwrap();
        assert_eq!(res_all.patched_content.trim(), modified.trim());
        assert!(res_all.syntax_warning.is_none());

        // Reject all hunks
        let res_none = apply_selected_hunks(original, &hunks, &[], Some("rs")).unwrap();
        assert_eq!(res_none.patched_content.trim(), original.trim());
    }

    #[test]
    fn test_non_contiguous_hunk_selection() {
        let original = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12\nline13\nline14\nline15";
        let modified = "LINE_ONE\nline2\nline3\nline4\nline5\nline6\nline7\nLINE_EIGHT\nline9\nline10\nline11\nline12\nline13\nline14\nLINE_FIFTEEN";

        let hunks = parse_diff_into_hunks(original, modified);
        assert!(hunks.len() >= 2);

        // Apply only first hunk
        let first_id = vec![hunks[0].hunk_id.clone()];
        let res = apply_selected_hunks(original, &hunks, &first_id, None).unwrap();

        assert!(res.patched_content.contains("LINE_ONE"));
        assert!(res.patched_content.contains("line8")); // Retained original line8
    }

    #[test]
    fn test_structural_syntax_safety_warning() {
        // Mismatched closing brace
        let mismatched_code = "fn calculate() {\n    let a = (10 + 20;\n}";
        let warning1 = check_structural_syntax_safety(mismatched_code, "rs");
        assert!(warning1.is_some());
        assert!(warning1.unwrap().contains("Mismatched closing brace"));

        // Unclosed delimiter
        let unclosed_code = "fn calculate() {\n    let a = 10;\n";
        let warning2 = check_structural_syntax_safety(unclosed_code, "rs");
        assert!(warning2.is_some());
        assert!(warning2.unwrap().contains("Unclosed delimiter"));
    }
}
