//! In-Memory Byte-Span Replacement & Formatter Pipeline.

#![forbid(unsafe_code)]

use crate::types::{RemediationEdit, RemediationKind};

/// In-memory byte-span replacement engine preserving alignment and whitespace.
pub struct PatchStrategy;

impl PatchStrategy {
    /// Apply an ordered set of non-overlapping byte replacements to source code.
    pub fn apply_edits(source: &str, mut edits: Vec<RemediationEdit>) -> String {
        // Sort edits in descending order of byte_start to preserve index offsets
        edits.sort_by_key(|b| std::cmp::Reverse(b.byte_start));

        let mut result = source.to_string();
        for edit in edits {
            if edit.byte_start <= result.len()
                && edit.byte_end <= result.len()
                && edit.byte_start <= edit.byte_end
            {
                result.replace_range(edit.byte_start..edit.byte_end, &edit.replacement);
            }
        }

        result
    }

    /// Surgically insert text at a specific byte offset.
    pub fn insert_at(source: &str, byte_offset: usize, insertion: &str) -> String {
        let mut result = source.to_string();
        let offset = byte_offset.min(result.len());
        result.insert_str(offset, insertion);
        result
    }

    /// Create a replacement edit descriptor.
    pub fn create_edit(
        kind: RemediationKind,
        desc: impl Into<String>,
        start: usize,
        end: usize,
        replacement: impl Into<String>,
    ) -> RemediationEdit {
        RemediationEdit {
            kind,
            description: desc.into(),
            byte_start: start,
            byte_end: end,
            replacement: replacement.into(),
        }
    }
}
