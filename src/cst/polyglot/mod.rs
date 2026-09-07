//! Lossless Polyglot Green-Red Concrete Syntax Tree (CST) Core.
//!
//! Provides a unified, 100% safe Rust abstraction layer for multi-language AST parsing,
//! lossless trivia preservation, symbol extraction, control-flow analysis, and safety verification.

#![forbid(unsafe_code)]

pub mod apps;
pub mod drivers;
pub mod registry;
pub mod systems;
pub mod traits;
pub mod types;

pub use apps::*;
pub use drivers::*;
pub use registry::{UniversalDriverRegistry, GLOBAL_REGISTRY};
pub use systems::*;
pub use traits::{UniversalCstDriver, UniversalCstDriverExt};
pub use types::{
    CfgEdge, CfgEdgeKind, CfgNode, CfgNodeKind, ControlFlowGraph, CstParseError, GreenTreeRoot,
    InvariantBitset, LanguageId, TreeStats,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polyglot_registry_defaults_and_dispatch() {
        let reg = UniversalDriverRegistry::with_defaults();
        let registered = reg.registered_languages();
        assert!(registered.contains(&LanguageId::Rust));
        assert!(registered.contains(&LanguageId::Go));
        assert!(registered.contains(&LanguageId::TypeScript));
        assert!(registered.contains(&LanguageId::Python));
        assert!(registered.contains(&LanguageId::Dart));
        assert!(registered.contains(&LanguageId::Java));
        assert!(registered.contains(&LanguageId::Php));

        // Test Rust dispatch
        let rust_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let tree = reg.parse_lossless(rust_code, &LanguageId::Rust).expect("Rust parse success");
        assert_eq!(tree.source_lossless_text(), rust_code);

        let symbols = reg.extract_symbols(&tree);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");

        let cfg = reg.compute_cfg(&tree);
        assert!(cfg.cyclomatic_complexity() >= 1);

        let invariants = reg.check_invariants(&tree);
        assert!(invariants.is_clean());
    }

    #[test]
    fn test_lossless_roundtrip_all_polyglot_languages() {
        let reg = UniversalDriverRegistry::with_defaults();

        // 1. Rust
        let rust_src = "/// Doc comment\npub async fn run() -> Result<(), Error> {\n    let x = 42;\n    Ok(())\n}\n";
        let r_tree = reg.parse_lossless(rust_src, &LanguageId::Rust).unwrap();
        assert_eq!(r_tree.source_lossless_text(), rust_src);

        // 2. TypeScript / TSX
        let tsx_src = "import React, { useState } from 'react';\n\nexport function Counter() {\n    const [count, setCount] = useState(0);\n    return <button onClick={() => setCount(count + 1)}>{count}</button>;\n}\n";
        let tsx_tree = reg.parse_lossless(tsx_src, &LanguageId::Tsx).unwrap();
        assert_eq!(tsx_tree.source_lossless_text(), tsx_src);

        // 3. Python
        let py_src = "# Python service\nclass Worker:\n    def __init__(self, name: str):\n        self.name = name\n\n    def execute(self) -> int:\n        return 100\n";
        let py_tree = reg.parse_lossless(py_src, &LanguageId::Python).unwrap();
        assert_eq!(py_tree.source_lossless_text(), py_src);

        // 4. Go
        let go_src = "package main\n\nimport \"fmt\"\n\ntype Service struct {\n    id int\n}\n\nfunc (s *Service) Start() {\n    go fmt.Println(\"started\")\n}\n";
        let go_tree = reg.parse_lossless(go_src, &LanguageId::Go).unwrap();
        assert_eq!(go_tree.source_lossless_text(), go_src);

        // 5. C++
        let cpp_src = "#include <iostream>\n\nclass Vector3 {\npublic:\n    float x, y, z;\n    Vector3(float x, float y, float z) : x(x), y(y), z(z) {}\n};\n\nint main() {\n    return 0;\n}\n";
        let cpp_tree = reg.parse_lossless(cpp_src, &LanguageId::Cpp).unwrap();
        assert_eq!(cpp_tree.source_lossless_text(), cpp_src);

        // 6. Dart
        let dart_src = "import 'package:flutter/material.dart';\n\nclass AppCard extends StatelessWidget {\n    final String title;\n    const AppCard({required this.title});\n}\n";
        let dart_tree = reg.parse_lossless(dart_src, &LanguageId::Dart).unwrap();
        assert_eq!(dart_tree.source_lossless_text(), dart_src);

        // 7. Java
        let java_src = "package com.locus.core;\n\npublic class EngineService {\n    private int status;\n    public void initialize() {\n        this.status = 1;\n    }\n}\n";
        let java_tree = reg.parse_lossless(java_src, &LanguageId::Java).unwrap();
        assert_eq!(java_tree.source_lossless_text(), java_src);

        // 8. PHP
        let php_src = "<?php\n\nnamespace App\\Services;\n\nclass AuthService {\n    public function login(string $user): bool {\n        return true;\n    }\n}\n";
        let php_tree = reg.parse_lossless(php_src, &LanguageId::Php).unwrap();
        assert_eq!(php_tree.source_lossless_text(), php_src);
    }

    #[test]
    fn test_polyglot_symbol_extraction_accuracy() {
        let reg = UniversalDriverRegistry::with_defaults();

        // TSX Components & Functions
        let code = "export function Header() {}\nexport const Footer = () => {};\nexport interface NavProps {}\nexport type Route = string;";
        let tree = reg.parse_lossless(code, &LanguageId::Tsx).unwrap();
        let symbols = reg.extract_symbols(&tree);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"Header"));
        assert!(names.contains(&"Footer"));
        assert!(names.contains(&"NavProps"));
        assert!(names.contains(&"Route"));

        // Go functions and structs
        let go_code = "type UserManager struct {}\ntype AuthProvider interface {}\nfunc NewUserManager() *UserManager {}";
        let go_tree = reg.parse_lossless(go_code, &LanguageId::Go).unwrap();
        let go_symbols = reg.extract_symbols(&go_tree);
        let go_names: Vec<&str> = go_symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(go_names.contains(&"UserManager"));
        assert!(go_names.contains(&"AuthProvider"));
        assert!(go_names.contains(&"NewUserManager"));
    }

    #[test]
    fn test_polyglot_cfg_branching_and_cyclomatic_complexity() {
        let reg = UniversalDriverRegistry::with_defaults();

        let branchy_rust = "fn compute(x: i32) -> i32 {\n    if x > 0 {\n        return 1;\n    } else {\n        return 0;\n    }\n}";
        let tree = reg.parse_lossless(branchy_rust, &LanguageId::Rust).unwrap();
        let cfg = reg.compute_cfg(&tree);

        assert_eq!(cfg.cyclomatic_complexity(), 2);
        assert!(!cfg.has_dead_code());
        assert!(cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::BranchCondition));
        assert!(cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::Return));
    }

    #[test]
    fn test_polyglot_invariant_bitset_detection() {
        let reg = UniversalDriverRegistry::with_defaults();

        // Code with div by zero and unsafe unwrap
        let unsafe_code = "pub fn calc(a: i32, b: i32) -> i32 { let opt = Some(42); let val = opt.unwrap(); a / b }";
        let tree = reg.parse_lossless(unsafe_code, &LanguageId::Rust).unwrap();
        let invariants = reg.check_invariants(&tree);

        assert!(!invariants.is_clean());
        assert!(invariants.has_violation(2) || invariants.has_violation(4));
    }
}
