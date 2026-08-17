//! Deterministic External Reasoning & Symbolic Constraint Engine
//!
//! Provides strict AST signature extraction to enforce non-hallucinatory boundaries,
//! language-aware edge case injection, and pre-verification self-repair for SEARCH/REPLACE blocks.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKind {
    Function,
    Struct,
    Interface,
    TypeAlias,
    Enum,
    Trait,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolicConstraint {
    pub kind: ConstraintKind,
    pub name: String,
    pub signature: String,
    pub context_hint: String,
}

pub struct DeterministicReasoningEngine;

impl DeterministicReasoningEngine {
    /// Extracts symbolic signatures and types from code with a maximum byte ceiling
    pub fn extract_symbolic_constraints(
        code: &str,
        lang: &str,
        max_bytes: usize,
    ) -> Vec<SymbolicConstraint> {
        let mut constraints = Vec::new();
        let target_code = if code.len() > max_bytes {
            &code[..max_bytes]
        } else {
            code
        };

        let lang_lower = lang.to_lowercase();

        match lang_lower.as_str() {
            "rust" | "rs" => {
                Self::extract_rust_constraints(target_code, &mut constraints);
            }
            "typescript" | "javascript" | "ts" | "js" | "tsx" | "jsx" => {
                Self::extract_ts_constraints(target_code, &mut constraints);
            }
            "python" | "py" => {
                Self::extract_python_constraints(target_code, &mut constraints);
            }
            "go" | "golang" => {
                Self::extract_go_constraints(target_code, &mut constraints);
            }
            "cpp" | "c++" | "c" | "h" | "hpp" => {
                Self::extract_cpp_constraints(target_code, &mut constraints);
            }
            _ => {
                // Generic extraction
                Self::extract_generic_constraints(target_code, &mut constraints);
            }
        }

        constraints
    }

    fn extract_rust_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let fn_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^\)]*\))?\s+)?(?:async\s+)?(?:const\s+)?fn\s+([a-zA-Z0-9_]+)\s*(\([^\)]*\))(?:\s*->\s*[^\r\n\{;]+)?").unwrap();
        for cap in fn_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full_sig = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Function,
                name,
                signature: full_sig,
                context_hint: "Rust function signature".to_string(),
            });
        }

        let struct_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^\)]*\))?\s+)?struct\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in struct_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Struct,
                name,
                signature: full,
                context_hint: "Rust struct definition".to_string(),
            });
        }

        let enum_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^\)]*\))?\s+)?enum\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in enum_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Enum,
                name,
                signature: full,
                context_hint: "Rust enum definition".to_string(),
            });
        }

        let trait_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^\)]*\))?\s+)?trait\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in trait_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Trait,
                name,
                signature: full,
                context_hint: "Rust trait definition".to_string(),
            });
        }
    }

    fn extract_ts_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let fn_re = Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z0-9_]+)\s*(\([^\)]*\))(?:\s*:\s*[^\r\n\{;]+)?").unwrap();
        for cap in fn_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full_sig = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Function,
                name,
                signature: full_sig,
                context_hint: "TypeScript function signature".to_string(),
            });
        }

        let interface_re = Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in interface_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Interface,
                name,
                signature: full,
                context_hint: "TypeScript interface declaration".to_string(),
            });
        }

        let type_re = Regex::new(r"(?m)^\s*(?:export\s+)?type\s+([a-zA-Z0-9_]+)\s*=").unwrap();
        for cap in type_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::TypeAlias,
                name,
                signature: full,
                context_hint: "TypeScript type alias".to_string(),
            });
        }
    }

    fn extract_python_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let fn_re = Regex::new(r"(?m)^\s*(?:async\s+)?def\s+([a-zA-Z0-9_]+)\s*(\([^\)]*\))(?:\s*->\s*[^\r\n:]+)?").unwrap();
        for cap in fn_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full_sig = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Function,
                name,
                signature: full_sig,
                context_hint: "Python function signature".to_string(),
            });
        }

        let class_re = Regex::new(r"(?m)^\s*class\s+([a-zA-Z0-9_]+)(?:\([^\)]*\))?").unwrap();
        for cap in class_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Struct,
                name,
                signature: full,
                context_hint: "Python class definition".to_string(),
            });
        }
    }

    fn extract_go_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let fn_re = Regex::new(r"(?m)^\s*func\s+(\([^\)]*\)\s+)?([a-zA-Z0-9_]+)\s*(\([^\)]*\))(\s*[^\s\{]+)?").unwrap();
        for cap in fn_re.captures_iter(code) {
            let name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full_sig = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Function,
                name,
                signature: full_sig,
                context_hint: "Go function signature".to_string(),
            });
        }

        let type_re = Regex::new(r"(?m)^\s*type\s+([a-zA-Z0-9_]+)\s+(struct|interface)").unwrap();
        for cap in type_re.captures_iter(code) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let kind_str = cap.get(2).map(|m| m.as_str()).unwrap_or("struct");
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            let kind = if kind_str == "interface" {
                ConstraintKind::Interface
            } else {
                ConstraintKind::Struct
            };
            out.push(SymbolicConstraint {
                kind,
                name,
                signature: full,
                context_hint: format!("Go {} definition", kind_str),
            });
        }
    }

    fn extract_cpp_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let class_re = Regex::new(r"(?m)^\s*(class|struct)\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in class_re.captures_iter(code) {
            let name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Struct,
                name,
                signature: full,
                context_hint: "C++ class/struct definition".to_string(),
            });
        }
    }

    fn extract_generic_constraints(code: &str, out: &mut Vec<SymbolicConstraint>) {
        let fn_re = Regex::new(r"(?m)^\s*(pub\s+|export\s+)?(function|def|fn)\s+([a-zA-Z0-9_]+)").unwrap();
        for cap in fn_re.captures_iter(code) {
            let name = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
            let full = cap.get(0).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            out.push(SymbolicConstraint {
                kind: ConstraintKind::Function,
                name,
                signature: full,
                context_hint: "Generic function signature".to_string(),
            });
        }
    }

    /// Injects language-specific edge cases and pitfalls into system prompt directives
    pub fn inject_language_edge_cases(lang: &str) -> Vec<String> {
        let lang_lower = lang.to_lowercase();
        match lang_lower.as_str() {
            "rust" | "rs" => vec![
                "Borrow Checker: Avoid simultaneous mutable and immutable borrows. Prefer scoped blocks or clones if necessary.".to_string(),
                "Panic Safety: Do NOT use .unwrap() or .expect() on user input paths; use '?' operator or handle Result/Option gracefully.".to_string(),
                "Thread Concurrency: Ensure types sent across async boundaries or threads implement Send + Sync + 'static.".to_string(),
                "Bounds Safety: Guard against slice and vector index out-of-bounds panics; prefer .get() or iterators.".to_string(),
            ],
            "typescript" | "javascript" | "ts" | "js" | "tsx" | "jsx" => vec![
                "Null Safety: Always guard against 'undefined' / 'null' with optional chaining (?.) and nullish coalescing (??).".to_string(),
                "Async/Promise Safety: Wrap await statements in try/catch or ensure unhandled Promise rejections are bubbled properly.".to_string(),
                "State Mutation: Do not mutate React state directly; use immutable spread updates (...prev).".to_string(),
                "Type Narrowing: Use discriminator fields or type guards (typeof, instanceof) when handling union types.".to_string(),
            ],
            "python" | "py" => vec![
                "NoneType Traps: Always check if variables are None before accessing attributes or calling methods.".to_string(),
                "Default Mutable Arguments: Never use mutable defaults (e.g. def fn(arg=[])); use None and initialize inside.".to_string(),
                "KeyError Protection: Use dict.get(key, default) instead of direct indexing unless existence is guaranteed.".to_string(),
                "Exception Specificity: Catch explicit Exception types (e.g. ValueError, IOError); avoid bare 'except:'.".to_string(),
            ],
            "go" | "golang" => vec![
                "Nil Pointer Dereference: Verify pointer and interface variables are non-nil before method calls.".to_string(),
                "Error Checking: Never ignore returned errors (_); always evaluate 'if err != nil'.".to_string(),
                "Goroutine Leaks: Ensure context cancellation (ctx.Done()) is checked to avoid hanging background goroutines.".to_string(),
                "Channel Deadlocks: Match channel send/receive capacity to avoid blocking unbuffered channels.".to_string(),
            ],
            "cpp" | "c++" | "c" | "h" | "hpp" => vec![
                "Memory Safety & RAII: Use smart pointers (std::unique_ptr, std::shared_ptr) instead of raw new/delete.".to_string(),
                "Nullptr Validation: Verify pointer validity before dereferencing.".to_string(),
                "Out of Bounds: Use std::array/std::vector with .at() or verify size before pointer arithmetic.".to_string(),
            ],
            _ => vec![
                "Resource Cleanup: Ensure files, sockets, and memory handles are properly released in error paths.".to_string(),
                "Boundary Validation: Validate inputs and array indices before processing.".to_string(),
            ],
        }
    }

    /// Pre-checks SEARCH/REPLACE blocks for syntax slips, typos in markers, and bracket balance,
    /// returning the self-repaired block while preserving indentation strictly.
    pub fn self_repair_search_replace(
        block_text: &str,
        file_content: &str,
    ) -> Result<String, String> {
        let trimmed = block_text.trim();
        if trimmed.is_empty() {
            return Err("Empty SEARCH/REPLACE block".to_string());
        }

        // 1. Repair minor typo variations in SEARCH/REPLACE markers
        let mut repaired = block_text.to_string();
        // Fix <<<<<< SEARCH or <<<<<<<< SEARCH -> <<<<<<< SEARCH
        let marker_start_re = Regex::new(r"(?m)^<{4,10}\s*SEARCH").unwrap();
        repaired = marker_start_re.replace_all(&repaired, "<<<<<<< SEARCH").to_string();

        // Fix ====== or ======== -> =======
        let marker_mid_re = Regex::new(r"(?m)^={4,10}\s*$").unwrap();
        repaired = marker_mid_re.replace_all(&repaired, "=======").to_string();

        // Fix >>>>>> or >>>>>>>> REPLACE -> >>>>>>> REPLACE
        let marker_end_re = Regex::new(r"(?m)^>{4,10}\s*(REPLACE)?").unwrap();
        repaired = marker_end_re.replace_all(&repaired, ">>>>>>> REPLACE").to_string();

        // 2. Validate markers exist
        let search_pos = repaired.find("<<<<<<< SEARCH");
        let divider_pos = repaired.find("=======");
        let replace_pos = repaired.find(">>>>>>> REPLACE");

        let (s_pos, d_pos, r_pos) = match (search_pos, divider_pos, replace_pos) {
            (Some(s), Some(d), Some(r)) if s < d && d < r => (s, d, r),
            _ => return Err("Malformed SEARCH/REPLACE block: Missing or out-of-order markers".to_string()),
        };

        // 3. Extract parts
        let search_content = &repaired[s_pos + "<<<<<<< SEARCH".len()..d_pos];
        let replace_content = &repaired[d_pos + "=======".len()..r_pos];

        // 4. Bracket balance repair on replacement content
        let mut open_braces = 0i32;
        let mut open_parens = 0i32;
        let mut open_brackets = 0i32;

        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for ch in replace_content.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
                continue;
            }
            if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
                continue;
            }
            if in_single_quote || in_double_quote {
                continue;
            }

            match ch {
                '{' => open_braces += 1,
                '}' => open_braces -= 1,
                '(' => open_parens += 1,
                ')' => open_parens -= 1,
                '[' => open_brackets += 1,
                ']' => open_brackets -= 1,
                _ => {}
            }
        }

        let mut repaired_replace = replace_content.to_string();
        // If missing closing brackets, append them before >>>>>>> REPLACE
        if open_braces > 0 {
            repaired_replace.push_str(&"\n}".repeat(open_braces as usize));
        }
        if open_parens > 0 {
            repaired_replace.push_str(&")".repeat(open_parens as usize));
        }
        if open_brackets > 0 {
            repaired_replace.push_str(&"]".repeat(open_brackets as usize));
        }

        // 5. Verify SEARCH content exists in target file with indentation match
        let raw_search = search_content.trim_matches(|c| c == '\r' || c == '\n');
        if !file_content.is_empty() && !file_content.contains(raw_search) {
            // Check if trimmed lines match
            let search_lines: Vec<&str> = raw_search.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            let mut found = false;
            if !search_lines.is_empty() {
                let file_lines: Vec<&str> = file_content.lines().collect();
                for i in 0..file_lines.len() {
                    if i + search_lines.len() <= file_lines.len() {
                        let matches = (0..search_lines.len()).all(|j| file_lines[i + j].trim() == search_lines[j]);
                        if matches {
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found && !raw_search.is_empty() {
                // Return warning/error if search block cannot be located anywhere in target file
                return Err(format!(
                    "SEARCH block does not match target file content: '{}'",
                    raw_search.lines().next().unwrap_or("")
                ));
            }
        }

        let final_block = format!(
            "<<<<<<< SEARCH\n{}\n=======\n{}\n>>>>>>> REPLACE",
            search_content.trim_matches(|c| c == '\r' || c == '\n'),
            repaired_replace.trim_matches(|c| c == '\r' || c == '\n')
        );

        Ok(final_block)
    }

    /// Assembles an enforced prompt directive combining symbolic constraints and language edge cases
    pub fn build_constrained_prompt(goal: &str, reference_code: &str, lang: &str) -> String {
        let constraints = Self::extract_symbolic_constraints(reference_code, lang, 100_000);
        let edge_cases = Self::inject_language_edge_cases(lang);

        let mut prompt = format!(
            "### OBJECTIVE\n{}\n\n### LANGUAGE RUNTIME: {}\n\n",
            goal, lang
        );

        if !constraints.is_empty() {
            prompt.push_str("### ENFORCED SYMBOLIC CONSTRAINTS (DO NOT INVENT TYPES OR MODIFY SIGNATURES):\n");
            for c in constraints.iter().take(15) {
                prompt.push_str(&format!("- [{:?}] `{}` ({})\n", c.kind, c.signature, c.context_hint));
            }
            prompt.push('\n');
        }

        if !edge_cases.is_empty() {
            prompt.push_str("### MANDATORY EDGE CASE DIRECTIVES:\n");
            for ec in edge_cases {
                prompt.push_str(&format!("- {}\n", ec));
            }
            prompt.push('\n');
        }

        prompt.push_str("### OUTPUT FORMAT RULES:\nUse strict `<<<<<<< SEARCH ... ======= ... >>>>>>> REPLACE` blocks exclusively for modifications.\n");
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_symbolic_constraints_rust() {
        let code = r#"
            pub struct EngineConfig {
                pub max_tokens: usize,
            }

            pub enum Status {
                Active,
                Idle,
            }

            pub trait Runner {
                fn run(&self) -> bool;
            }

            pub async fn execute_task(ctx: &str, count: u32) -> Result<String, Error> {
                Ok("done".into())
            }
        "#;

        let constraints = DeterministicReasoningEngine::extract_symbolic_constraints(code, "rust", 100_000);
        assert_eq!(constraints.len(), 5);

        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Struct && c.name == "EngineConfig"));
        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Enum && c.name == "Status"));
        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Trait && c.name == "Runner"));
        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Function && c.name == "execute_task"));
    }

    #[test]
    fn test_extract_symbolic_constraints_typescript() {
        let code = r#"
            export interface UserProfile {
                id: string;
                name: string;
            }

            export type AuthToken = string;

            export async function fetchUser(userId: string): Promise<UserProfile> {
                return { id: userId, name: "test" };
            }
        "#;

        let constraints = DeterministicReasoningEngine::extract_symbolic_constraints(code, "typescript", 100_000);
        assert_eq!(constraints.len(), 3);

        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Interface && c.name == "UserProfile"));
        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::TypeAlias && c.name == "AuthToken"));
        assert!(constraints.iter().any(|c| c.kind == ConstraintKind::Function && c.name == "fetchUser"));
    }

    #[test]
    fn test_inject_language_edge_cases_rust_ts_py() {
        let rust_cases = DeterministicReasoningEngine::inject_language_edge_cases("rust");
        assert!(rust_cases.iter().any(|c| c.contains("Borrow Checker")));
        assert!(rust_cases.iter().any(|c| c.contains("Panic Safety")));

        let ts_cases = DeterministicReasoningEngine::inject_language_edge_cases("typescript");
        assert!(ts_cases.iter().any(|c| c.contains("Null Safety")));
        assert!(ts_cases.iter().any(|c| c.contains("Async/Promise")));

        let py_cases = DeterministicReasoningEngine::inject_language_edge_cases("python");
        assert!(py_cases.iter().any(|c| c.contains("NoneType")));
        assert!(py_cases.iter().any(|c| c.contains("Default Mutable Arguments")));
    }

    #[test]
    fn test_self_repair_search_replace_unbalanced_braces() {
        let file = "fn main() {\n    println!(\"hello\");\n}";
        // LLM generated replacement missing closing brace
        let raw_block = "<<<<<<< SEARCH\n    println!(\"hello\");\n=======\n    if true {\n        println!(\"hello repaired\");\n>>>>>>> REPLACE";

        let repaired = DeterministicReasoningEngine::self_repair_search_replace(raw_block, file)
            .expect("Should repair unbalanced braces");

        assert!(repaired.contains("if true {"));
        assert!(repaired.contains("}")); // Auto-appended missing closing brace
    }

    #[test]
    fn test_self_repair_marker_typos() {
        let file = "let count = 0;";
        // Typo: <<<<<< SEARCH (6 instead of 7) and >>>>>>>> REPLACE (8 instead of 7)
        let typo_block = "<<<<<< SEARCH\nlet count = 0;\n======\nlet count = 1;\n>>>>>>>> REPLACE";

        let repaired = DeterministicReasoningEngine::self_repair_search_replace(typo_block, file)
            .expect("Should repair marker typos");

        assert!(repaired.starts_with("<<<<<<< SEARCH"));
        assert!(repaired.contains("======="));
        assert!(repaired.ends_with(">>>>>>> REPLACE"));
    }
}
