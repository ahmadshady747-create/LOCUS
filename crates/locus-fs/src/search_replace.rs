use serde::{Deserialize, Serialize};

/// Represents a discrete Search/Replace block directive emitted by an LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchReplaceBlock {
    pub search: String,
    pub replace: String,
    pub file_path_hint: Option<String>,
}

/// Parses all `<<<<<<< SEARCH ... ======= ... >>>>>>> REPLACE` blocks from text
pub fn parse_search_replace_blocks(text: &str) -> Vec<SearchReplaceBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if is_search_start(line) {
            let mut search_lines = Vec::new();
            i += 1;

            // Collect search lines until divider =======
            while i < lines.len() && !is_divider(lines[i].trim()) {
                search_lines.push(lines[i]);
                i += 1;
            }

            if i < lines.len() && is_divider(lines[i].trim()) {
                i += 1;
                let mut replace_lines = Vec::new();

                // Collect replace lines until >>>>>>> REPLACE
                while i < lines.len() && !is_replace_end(lines[i].trim()) {
                    replace_lines.push(lines[i]);
                    i += 1;
                }

                let search = search_lines.join("\n");
                let replace = replace_lines.join("\n");

                blocks.push(SearchReplaceBlock {
                    search,
                    replace,
                    file_path_hint: None,
                });
            }
        }

        i += 1;
    }

    blocks
}

fn is_search_start(line: &str) -> bool {
    line.starts_with("<<<<<<< SEARCH") || line.starts_with("<<<<<<<SEARCH")
}

fn is_divider(line: &str) -> bool {
    line.starts_with("=======") || line == "==="
}

fn is_replace_end(line: &str) -> bool {
    line.starts_with(">>>>>>> REPLACE")
        || line.starts_with(">>>>>>>REPLACE")
        || line.starts_with(">>>>>>>")
}

/// Applies a sequence of SearchReplaceBlocks to an original document with multi-tier matching
/// (Exact -> Trimmed -> Indentation Normalized) and strict Ambiguity Protection.
pub fn apply_search_replace_blocks(
    original: &str,
    blocks: &[SearchReplaceBlock],
) -> Result<(String, usize), String> {
    if blocks.is_empty() {
        return Ok((original.to_string(), 0));
    }

    let mut current_text = original.to_string();
    let mut applied_count = 0;

    for (idx, block) in blocks.iter().enumerate() {
        let block_num = idx + 1;
        let search_target = &block.search;

        if search_target.trim().is_empty() {
            continue;
        }

        // Tier 1: Exact Substring Matching
        let matches = find_exact_occurrences(&current_text, search_target);
        if matches.len() == 1 {
            let start = matches[0];
            let end = start + search_target.len();
            current_text = format!(
                "{}{}{}",
                &current_text[..start],
                &block.replace,
                &current_text[end..]
            );
            applied_count += 1;
            continue;
        } else if matches.len() > 1 {
            return Err(format!(
                "Ambiguity Guard Error in Block #{}: SEARCH content matched {} distinct locations in file. Provide more unique surrounding context lines.",
                block_num, matches.len()
            ));
        }

        // Tier 2: Line-by-Line Trimmed Matching
        let line_matches = find_line_trimmed_occurrences(&current_text, search_target);
        if line_matches.len() == 1 {
            let (start_line, line_count) = line_matches[0];
            current_text = replace_line_range(&current_text, start_line, line_count, &block.replace);
            applied_count += 1;
            continue;
        } else if line_matches.len() > 1 {
            return Err(format!(
                "Ambiguity Guard Error in Block #{}: Trimmed search matched {} distinct locations in file. Add more surrounding context.",
                block_num, line_matches.len()
            ));
        }

        // Tier 3: Indentation-Normalized Matching
        let indent_matches = find_indentation_normalized_occurrences(&current_text, search_target);
        if indent_matches.len() == 1 {
            let (start_line, line_count) = indent_matches[0];
            current_text = replace_line_range(&current_text, start_line, line_count, &block.replace);
            applied_count += 1;
            continue;
        } else if indent_matches.len() > 1 {
            return Err(format!(
                "Ambiguity Guard Error in Block #{}: Indentation-normalized search matched {} distinct locations. Add more surrounding context.",
                block_num, indent_matches.len()
            ));
        }

        // If no matches found in any tier
        return Err(format!(
            "Failed to find match for SEARCH Block #{}:\n\"\"\"\n{}\n\"\"\"\nEnsure the code has not already been modified.",
            block_num, search_target
        ));
    }

    Ok((current_text, applied_count))
}

fn find_exact_occurrences(text: &str, target: &str) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut start = 0;
    while let Some(pos) = text[start..].find(target) {
        let abs_pos = start + pos;
        indices.push(abs_pos);
        start = abs_pos + target.len().max(1);
    }
    indices
}

fn find_line_trimmed_occurrences(text: &str, search: &str) -> Vec<(usize, usize)> {
    let text_lines: Vec<&str> = text.lines().collect();
    let search_lines: Vec<&str> = search.lines().collect();

    if search_lines.is_empty() || text_lines.len() < search_lines.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let window_len = search_lines.len();

    for i in 0..=(text_lines.len() - window_len) {
        let mut all_match = true;
        for j in 0..window_len {
            if text_lines[i + j].trim_end() != search_lines[j].trim_end() {
                all_match = false;
                break;
            }
        }
        if all_match {
            matches.push((i, window_len));
        }
    }

    matches
}

fn find_indentation_normalized_occurrences(text: &str, search: &str) -> Vec<(usize, usize)> {
    let text_lines: Vec<&str> = text.lines().collect();
    let search_lines: Vec<&str> = search.lines().collect();

    if search_lines.is_empty() || text_lines.len() < search_lines.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let window_len = search_lines.len();

    for i in 0..=(text_lines.len() - window_len) {
        let mut all_match = true;
        for j in 0..window_len {
            if text_lines[i + j].trim() != search_lines[j].trim() {
                all_match = false;
                break;
            }
        }
        if all_match {
            matches.push((i, window_len));
        }
    }

    matches
}

fn replace_line_range(text: &str, start_line: usize, line_count: usize, replacement: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out_lines = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if idx == start_line {
            if !replacement.is_empty() {
                out_lines.push(replacement.to_string());
            }
        } else if idx > start_line && idx < start_line + line_count {
            // skip lines in replaced range
            continue;
        } else {
            out_lines.push(line.to_string());
        }
    }

    let mut res = out_lines.join("\n");
    if text.ends_with('\n') {
        res.push('\n');
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_apply_exact_search_replace() {
        let original = r#"fn calculate(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum * 2
}"#;

        let llm_output = r#"Here is the fix:
<<<<<<< SEARCH
    let sum = a + b;
    sum * 2
=======
    let sum = a + b;
    sum * 3
>>>>>>> REPLACE
Done."#;

        let blocks = parse_search_replace_blocks(llm_output);
        assert_eq!(blocks.len(), 1);

        let (new_text, applied) = apply_search_replace_blocks(original, &blocks).unwrap();
        assert_eq!(applied, 1);
        assert!(new_text.contains("sum * 3"));
        assert!(!new_text.contains("sum * 2"));
    }

    #[test]
    fn test_indentation_normalized_match() {
        let original = "    pub fn connect() {\n        info!(\"connecting\");\n    }";
        let llm_block = SearchReplaceBlock {
            search: "pub fn connect() {\n    info!(\"connecting\");\n}".to_string(), // different indent
            replace: "    pub fn connect_v2() {\n        info!(\"connected v2\");\n    }".to_string(),
            file_path_hint: None,
        };

        let (new_text, applied) = apply_search_replace_blocks(original, &[llm_block]).unwrap();
        assert_eq!(applied, 1);
        assert!(new_text.contains("connect_v2"));
    }

    #[test]
    fn test_ambiguity_guard_triggers_error_on_duplicates() {
        let original = "let x = 1;\nlet y = 2;\nlet x = 1;\nlet z = 3;";
        let llm_block = SearchReplaceBlock {
            search: "let x = 1;".to_string(),
            replace: "let x = 99;".to_string(),
            file_path_hint: None,
        };

        let res = apply_search_replace_blocks(original, &[llm_block]);
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("Ambiguity Guard Error"));
        assert!(err_msg.contains("matched 2 distinct locations"));
    }
}
