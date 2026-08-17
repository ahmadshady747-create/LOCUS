//! Compiler and runtime error resolver radar.
//!
//! Parses compiler error codes and tracebacks, provides idiomatic solutions,
//! and synthesizes negative memory anti-patterns for injection into the ADR ledger.

use crate::types::ResolvedErrorSolution;
use regex::Regex;
use tracing::info;

pub struct IssueResolverRadar;

impl IssueResolverRadar {
    /// Analyzes an error snippet, extracts error codes, and resolves root-cause explanations and fixes.
    pub fn resolve_error(snippet: &str) -> ResolvedErrorSolution {
        let snippet_clean = snippet.trim();

        // 1. Check for Rust error code: E0382, E0502, E0599, E0277, etc.
        let re_rust = Regex::new(r"\b(E\d{4})\b").unwrap();
        if let Some(caps) = re_rust.captures(snippet_clean) {
            let code = caps.get(1).unwrap().as_str();
            return Self::resolve_rust_error(code, snippet_clean);
        }

        // 2. Check for TypeScript error code: TS2304, TS2345, TS2322, TS7006, TS18048, etc.
        let re_ts = Regex::new(r"\b(TS\d{4,5})\b").unwrap();
        if let Some(caps) = re_ts.captures(snippet_clean) {
            let code = caps.get(1).unwrap().as_str();
            return Self::resolve_typescript_error(code, snippet_clean);
        }

        // 3. Check for Python Exception: TypeError, AttributeError, ImportError, etc.
        let re_py = Regex::new(r"\b(TypeError|AttributeError|ImportError|ModuleNotFoundError|KeyError|IndexError|ValueError)\b").unwrap();
        if let Some(caps) = re_py.captures(snippet_clean) {
            let exc = caps.get(1).unwrap().as_str();
            return Self::resolve_python_error(exc, snippet_clean);
        }

        // 4. Fallback Generic diagnosis
        Self::resolve_generic_error(snippet_clean)
    }

    fn resolve_rust_error(code: &str, raw: &str) -> ResolvedErrorSolution {
        info!("Resolving Rust compiler error: {}", code);
        match code {
            "E0382" => ResolvedErrorSolution {
                error_code: "E0382".to_string(),
                error_title: "Borrow of Moved Value".to_string(),
                language: "Rust".to_string(),
                explanation: "You are attempting to use a variable or value after its ownership has already been transferred (moved) to another binding, closure, or function.".to_string(),
                recommended_fix_markdown: "```rust\n// Solution 1: Clone before move (if Type implements Clone)\nlet val_clone = val.clone();\nconsume(val_clone);\nuse_again(&val);\n\n// Solution 2: Pass by reference instead of value\nfn consume(val: &MyStruct) { ... }\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Moving non-Copy types across closures or iterative loops without .clone() or borrowing.".to_string(),
                references: vec![
                    "https://doc.rust-lang.org/error_codes/E0382.html".to_string(),
                    "https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html".to_string(),
                ],
            },
            "E0502" => ResolvedErrorSolution {
                error_code: "E0502".to_string(),
                error_title: "Cannot Borrow as Mutable because Borrowed as Immutable".to_string(),
                language: "Rust".to_string(),
                explanation: "Rust's aliasing XOR mutability rule prohibits having an active mutable reference while immutable references still exist in the same scope.".to_string(),
                recommended_fix_markdown: "```rust\n// Solution: Limit the lifetime of the immutable borrow before mutating\n{\n    let view = &data.items;\n    println!(\"{:?}\", view);\n} // immutable borrow ends here\n\ndata.items.push(new_item); // now safe to borrow mutably\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Holding a slice or reference while calling methods that reallocate or mutate the underlying collection.".to_string(),
                references: vec![
                    "https://doc.rust-lang.org/error_codes/E0502.html".to_string(),
                ],
            },
            "E0599" => ResolvedErrorSolution {
                error_code: "E0599".to_string(),
                error_title: "No Method or Associated Item Found in Scope".to_string(),
                language: "Rust".to_string(),
                explanation: "The method is not implemented for this type, or the trait defining the method has not been brought into scope with a `use` statement.".to_string(),
                recommended_fix_markdown: "```rust\n// Ensure the trait providing the method is imported:\nuse futures::StreamExt; // for .next(), .map(), etc.\nuse std::io::Read;      // for .read(), .read_to_string()\nuse tokio::io::AsyncWriteExt; // for .write_all()\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Calling trait methods without importing the extension trait in file headers.".to_string(),
                references: vec![
                    "https://doc.rust-lang.org/error_codes/E0599.html".to_string(),
                ],
            },
            "E0277" => ResolvedErrorSolution {
                error_code: "E0277".to_string(),
                error_title: "Trait Bound Not Satisfied".to_string(),
                language: "Rust".to_string(),
                explanation: "A generic function or struct requires a trait bound (e.g. `Send`, `Sync`, `Clone`, `Display`, `Serialize`) that the provided type does not implement.".to_string(),
                recommended_fix_markdown: "```rust\n// Derive or implement the missing trait:\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct MyType {\n    pub id: String,\n}\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Passing types to async tokio tasks without #[derive(Clone, Send, Sync)].".to_string(),
                references: vec![
                    "https://doc.rust-lang.org/error_codes/E0277.html".to_string(),
                ],
            },
            _ => ResolvedErrorSolution {
                error_code: code.to_string(),
                error_title: format!("Rust Compiler Error {}", code),
                language: "Rust".to_string(),
                explanation: format!("Rust compiler error {}: Review compiler error diagnostic details.", code),
                recommended_fix_markdown: format!("```rust\n// Review snippet:\n{}\n```", raw),
                negative_memory_pattern: format!("Anti-pattern causing Rust error {}", code),
                references: vec![format!("https://doc.rust-lang.org/error_codes/{}.html", code)],
            },
        }
    }

    fn resolve_typescript_error(code: &str, raw: &str) -> ResolvedErrorSolution {
        info!("Resolving TypeScript compiler error: {}", code);
        match code {
            "TS2304" => ResolvedErrorSolution {
                error_code: "TS2304".to_string(),
                error_title: "Cannot Find Name".to_string(),
                language: "TypeScript".to_string(),
                explanation: "A variable, type, or function identifier is referenced without being declared or imported.".to_string(),
                recommended_fix_markdown: "```typescript\n// Solution: Add the missing import statement:\nimport { MySymbol } from \"../types\";\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Using types or helper components without explicit module imports.".to_string(),
                references: vec!["https://www.typescriptlang.org/docs/handbook/modules.html".to_string()],
            },
            "TS2345" | "TS2322" => ResolvedErrorSolution {
                error_code: code.to_string(),
                error_title: "Type Mismatch / Not Assignable".to_string(),
                language: "TypeScript".to_string(),
                explanation: "The type of the expression provided is incompatible with the expected parameter or property signature.".to_string(),
                recommended_fix_markdown: "```typescript\n// Solution: Align property types or use optional chaining / type guards\ninterface ExpectedProps {\n  id: string;\n  optionalVal?: number;\n}\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Passing null/undefined or raw strings to strictly typed numeric/boolean interfaces.".to_string(),
                references: vec!["https://www.typescriptlang.org/docs/handbook/2/everyday-types.html".to_string()],
            },
            "TS18048" => ResolvedErrorSolution {
                error_code: "TS18048".to_string(),
                error_title: "Value is Possibly Undefined".to_string(),
                language: "TypeScript".to_string(),
                explanation: "Strict null checks detected that a variable may be undefined when accessed.".to_string(),
                recommended_fix_markdown: "```typescript\n// Solution: Use optional chaining or nullish coalescing:\nconst val = item?.property ?? \"default_value\";\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Direct member access on array find() or dictionary lookup results without null guard.".to_string(),
                references: vec!["https://www.typescriptlang.org/docs/handbook/release-notes/typescript-3-7.html#optional-chaining".to_string()],
            },
            _ => ResolvedErrorSolution {
                error_code: code.to_string(),
                error_title: format!("TypeScript Diagnostic {}", code),
                language: "TypeScript".to_string(),
                explanation: format!("TypeScript compiler error {}: Type mismatch or undeclared symbol.", code),
                recommended_fix_markdown: format!("```typescript\n// Diagnostic snippet:\n{}\n```", raw),
                negative_memory_pattern: format!("Anti-pattern causing TypeScript error {}", code),
                references: vec!["https://www.typescriptlang.org/docs/".to_string()],
            },
        }
    }

    fn resolve_python_error(exc: &str, raw: &str) -> ResolvedErrorSolution {
        info!("Resolving Python runtime exception: {}", exc);
        match exc {
            "TypeError" => ResolvedErrorSolution {
                error_code: "TypeError".to_string(),
                error_title: "Python TypeError".to_string(),
                language: "Python".to_string(),
                explanation: "An operation or function was applied to an object of an inappropriate type, or wrong arguments count was supplied.".to_string(),
                recommended_fix_markdown: "```python\n# Solution: Validate types and arguments\nif isinstance(val, (int, float)):\n    result = val + 10\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Implicit type conversion assumptions between strings, dicts, and integers.".to_string(),
                references: vec!["https://docs.python.org/3/library/exceptions.html#TypeError".to_string()],
            },
            "AttributeError" => ResolvedErrorSolution {
                error_code: "AttributeError".to_string(),
                error_title: "Python AttributeError".to_string(),
                language: "Python".to_string(),
                explanation: "Attempted to access an attribute or method that does not exist on the target object (often NoneType).".to_string(),
                recommended_fix_markdown: "```python\n# Solution: Guard against None objects\nif obj is not None and hasattr(obj, 'target_attr'):\n    val = obj.target_attr\n```".to_string(),
                negative_memory_pattern: "Anti-pattern: Chaining method calls on objects returned from search/filter without None verification.".to_string(),
                references: vec!["https://docs.python.org/3/library/exceptions.html#AttributeError".to_string()],
            },
            _ => ResolvedErrorSolution {
                error_code: exc.to_string(),
                error_title: format!("Python {}", exc),
                language: "Python".to_string(),
                explanation: format!("Python exception '{}': Check stack trace context.", exc),
                recommended_fix_markdown: format!("```python\n# Traceback snippet:\n{}\n```", raw),
                negative_memory_pattern: format!("Anti-pattern causing Python {}", exc),
                references: vec!["https://docs.python.org/3/library/exceptions.html".to_string()],
            },
        }
    }

    fn resolve_generic_error(raw: &str) -> ResolvedErrorSolution {
        ResolvedErrorSolution {
            error_code: "GENERIC_ERROR".to_string(),
            error_title: "Compiler / Runtime Diagnostic".to_string(),
            language: "Multi-Language".to_string(),
            explanation: "General compiler or runtime diagnostic detected. Review context and signatures.".to_string(),
            recommended_fix_markdown: format!("```\n{}\n```", raw),
            negative_memory_pattern: "Anti-pattern identified in compiler error output.".to_string(),
            references: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_rust_borrow_error() {
        let snippet = "error[E0382]: use of moved value: `buffer`\n  --> src/main.rs:12:9";
        let solution = IssueResolverRadar::resolve_error(snippet);
        assert_eq!(solution.error_code, "E0382");
        assert_eq!(solution.language, "Rust");
        assert!(solution.recommended_fix_markdown.contains("clone()"));
    }

    #[test]
    fn test_resolve_typescript_type_error() {
        let snippet = "src/components/App.tsx(45,7): error TS2345: Argument of type 'string' is not assignable to parameter of type 'number'.";
        let solution = IssueResolverRadar::resolve_error(snippet);
        assert_eq!(solution.error_code, "TS2345");
        assert_eq!(solution.language, "TypeScript");
    }

    #[test]
    fn test_resolve_python_attribute_error() {
        let snippet = "AttributeError: 'NoneType' object has no attribute 'get'";
        let solution = IssueResolverRadar::resolve_error(snippet);
        assert_eq!(solution.error_code, "AttributeError");
        assert_eq!(solution.language, "Python");
    }
}
