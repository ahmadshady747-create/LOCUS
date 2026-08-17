use locus_core::types::{DiffHunk, DiffLine, DiffLineType};

/// Computes discrete Diff Hunks between original and proposed text.
pub fn compute_hunks(original: &str, proposed: &str) -> Vec<DiffHunk> {
    let orig_lines: Vec<&str> = if original.is_empty() {
        Vec::new()
    } else {
        original.lines().collect()
    };
    let prop_lines: Vec<&str> = if proposed.is_empty() {
        Vec::new()
    } else {
        proposed.lines().collect()
    };

    if orig_lines == prop_lines {
        return Vec::new();
    }

    // Standard LCS-based Diff
    let lcs_matrix = compute_lcs(&orig_lines, &prop_lines);
    let raw_diff = build_raw_diff(&orig_lines, &prop_lines, &lcs_matrix);

    // Group into hunks with context
    group_into_hunks(raw_diff, 3)
}

/// Applies a single hunk to the original text, returning the newly modified text.
pub fn apply_single_hunk(original: &str, hunk: &DiffHunk) -> String {
    let orig_lines: Vec<&str> = if original.is_empty() {
        Vec::new()
    } else {
        original.lines().collect()
    };

    let mut result_lines: Vec<String> = Vec::new();
    let mut orig_idx = 0;

    // 1. Copy lines before the hunk
    let hunk_start = if hunk.old_start > 0 { hunk.old_start - 1 } else { 0 };
    while orig_idx < hunk_start && orig_idx < orig_lines.len() {
        result_lines.push(orig_lines[orig_idx].to_string());
        orig_idx += 1;
    }

    // 2. Apply the hunk lines
    for line in &hunk.lines {
        match line.line_type {
            DiffLineType::Context => {
                if orig_idx < orig_lines.len() {
                    result_lines.push(orig_lines[orig_idx].to_string());
                    orig_idx += 1;
                } else {
                    result_lines.push(line.content.clone());
                }
            }
            DiffLineType::Addition => {
                result_lines.push(line.content.clone());
            }
            DiffLineType::Deletion => {
                if orig_idx < orig_lines.len() {
                    orig_idx += 1; // skip deleted line from original
                }
            }
        }
    }

    // 3. Copy remaining lines after the hunk
    while orig_idx < orig_lines.len() {
        result_lines.push(orig_lines[orig_idx].to_string());
        orig_idx += 1;
    }

    let mut res = result_lines.join("\n");
    if original.ends_with('\n') || (original.is_empty() && !res.is_empty()) {
        res.push('\n');
    }
    res
}

/// Rejects a single hunk by discarding its proposed additions/deletions in proposed text.
pub fn reject_single_hunk(original: &str, proposed: &str, hunk: &DiffHunk) -> String {
    // When rejecting a hunk, we apply all OTHER hunks except this one to original
    let all_hunks = compute_hunks(original, proposed);
    let mut current = original.to_string();

    for h in all_hunks {
        if h.hunk_id != hunk.hunk_id {
            current = apply_single_hunk(&current, &h);
        }
    }

    current
}

#[derive(Debug, Clone)]
enum RawDiffOp<'a> {
    Equal(&'a str, usize, usize),    // line, old_no, new_no
    Insert(&'a str, usize),           // line, new_no
    Delete(&'a str, usize),           // line, old_no
}

fn compute_lcs(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let m = a.len();
    let n = b.len();
    let mut table = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..m {
        for j in 0..n {
            if a[i] == b[j] {
                table[i + 1][j + 1] = table[i][j] + 1;
            } else {
                table[i + 1][j + 1] = table[i + 1][j].max(table[i][j + 1]);
            }
        }
    }
    table
}

fn build_raw_diff<'a>(a: &[&'a str], b: &[&'a str], table: &[Vec<usize>]) -> Vec<RawDiffOp<'a>> {
    let mut ops = Vec::new();
    let mut i = a.len();
    let mut j = b.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            ops.push(RawDiffOp::Equal(a[i - 1], i, j));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || table[i][j - 1] >= table[i - 1][j]) {
            ops.push(RawDiffOp::Insert(b[j - 1], j));
            j -= 1;
        } else if i > 0 && (j == 0 || table[i][j - 1] < table[i - 1][j]) {
            ops.push(RawDiffOp::Delete(a[i - 1], i));
            i -= 1;
        }
    }

    ops.reverse();
    ops
}

fn group_into_hunks(ops: Vec<RawDiffOp>, context_radius: usize) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    if ops.is_empty() {
        return hunks;
    }

    // Identify indices with changes
    let mut change_indices = Vec::new();
    for (idx, op) in ops.iter().enumerate() {
        match op {
            RawDiffOp::Insert(..) | RawDiffOp::Delete(..) => change_indices.push(idx),
            RawDiffOp::Equal(..) => {}
        }
    }

    if change_indices.is_empty() {
        return hunks;
    }

    // Cluster change indices with context_radius * 2 threshold
    let mut clusters: Vec<(usize, usize)> = Vec::new();
    let mut current_cluster: Option<(usize, usize)> = None;

    for idx in change_indices {
        let start = idx.saturating_sub(context_radius);
        let end = (idx + context_radius + 1).min(ops.len());

        match current_cluster {
            None => current_cluster = Some((start, end)),
            Some((c_start, c_end)) => {
                if start <= c_end {
                    current_cluster = Some((c_start, end.max(c_end)));
                } else {
                    clusters.push((c_start, c_end));
                    current_cluster = Some((start, end));
                }
            }
        }
    }

    if let Some(c) = current_cluster {
        clusters.push(c);
    }

    // Build DiffHunk for each cluster
    for (hunk_idx, (start, end)) in clusters.into_iter().enumerate() {
        let slice = &ops[start..end];
        let mut lines = Vec::new();
        let mut old_start = 0;
        let mut old_lines = 0;
        let mut new_start = 0;
        let mut new_lines = 0;

        for op in slice {
            match op {
                RawDiffOp::Equal(c, o_num, n_num) => {
                    if old_start == 0 {
                        old_start = *o_num;
                    }
                    if new_start == 0 {
                        new_start = *n_num;
                    }
                    old_lines += 1;
                    new_lines += 1;
                    lines.push(DiffLine {
                        line_type: DiffLineType::Context,
                        content: c.to_string(),
                        old_line_no: Some(*o_num),
                        new_line_no: Some(*n_num),
                    });
                }
                RawDiffOp::Insert(c, n_num) => {
                    if new_start == 0 {
                        new_start = *n_num;
                    }
                    new_lines += 1;
                    lines.push(DiffLine {
                        line_type: DiffLineType::Addition,
                        content: c.to_string(),
                        old_line_no: None,
                        new_line_no: Some(*n_num),
                    });
                }
                RawDiffOp::Delete(c, o_num) => {
                    if old_start == 0 {
                        old_start = *o_num;
                    }
                    old_lines += 1;
                    lines.push(DiffLine {
                        line_type: DiffLineType::Deletion,
                        content: c.to_string(),
                        old_line_no: Some(*o_num),
                        new_line_no: None,
                    });
                }
            }
        }

        if old_start == 0 {
            old_start = 1;
        }
        if new_start == 0 {
            new_start = 1;
        }

        let header = format!("@@ -{},{} +{},{} @@", old_start, old_lines, new_start, new_lines);
        hunks.push(DiffHunk {
            hunk_id: format!("hunk-{}", hunk_idx + 1),
            old_start,
            old_lines,
            new_start,
            new_lines,
            header,
            lines,
        });
    }

    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_single_hunk() {
        let orig = "fn main() {\n    println!(\"hello\");\n}\n";
        let prop = "fn main() {\n    println!(\"hello world!\");\n}\n";

        let hunks = compute_hunks(orig, prop);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].hunk_id, "hunk-1");
        assert!(hunks[0].lines.iter().any(|l| l.line_type == DiffLineType::Addition && l.content.contains("world!")));
    }

    #[test]
    fn test_apply_single_hunk() {
        let orig = "fn a() {}\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nfn c() {}\n";
        let prop = "fn a() { println!(\"1\"); }\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nfn c() { println!(\"2\"); }\n";

        let hunks = compute_hunks(orig, prop);
        assert_eq!(hunks.len(), 2);

        // Apply only hunk 1
        let applied_1 = apply_single_hunk(orig, &hunks[0]);
        assert!(applied_1.contains("println!(\"1\");"));
        assert!(!applied_1.contains("println!(\"2\");"));

        // Recompute remaining hunks
        let remaining = compute_hunks(&applied_1, prop);
        assert_eq!(remaining.len(), 1);

        // Apply remaining hunk
        let applied_2 = apply_single_hunk(&applied_1, &remaining[0]);
        assert!(applied_2.contains("println!(\"1\");"));
        assert!(applied_2.contains("println!(\"2\");"));
    }

    #[test]
    fn test_reject_single_hunk() {
        let orig = "line1\nline2\nline3\n";
        let prop = "line1\nline2_modified\nline3\n";

        let hunks = compute_hunks(orig, prop);
        assert_eq!(hunks.len(), 1);

        let rejected = reject_single_hunk(orig, prop, &hunks[0]);
        assert_eq!(rejected.trim(), orig.trim());
    }
}
