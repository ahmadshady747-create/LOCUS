//! Concrete Polyglot CST Driver Implementations.
//!
//! Provides lossless parsing, symbol extraction, deterministic CFG computation,
//! and AST safety invariant checking across Systems and Application languages.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use regex::Regex;

use crate::cst::green::{GreenNode, GreenNodeBuilder, SyntaxKind, TextRange, TextSize};
use crate::cst::lexer::tokenize;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{
    CfgEdge, CfgEdgeKind, CfgNode, CfgNodeKind, ControlFlowGraph, CstParseError, GreenTreeRoot,
    InvariantBitset, LanguageId,
};
use crate::guard::AstGuard;
use crate::types::{ResolvedSymbol, SymbolKind};

// ---------------------------------------------------------------------------
// Helper: Lossless Tree Construction
// ---------------------------------------------------------------------------

/// Builds a lossless GreenNode tree from source tokens, grouping declarations when possible.
pub fn build_lossless_green(source: &str) -> Result<GreenNode, CstParseError> {
    if !AstGuard::check_delimiter_balance(source) {
        return Err(CstParseError::UnbalancedDelimiter {
            expected: '}',
            found: None,
            offset: source.len() as TextSize,
        });
    }

    let tokens = tokenize(source);
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::Root);

    let mut in_decl = false;
    let mut brace_depth = 0;

    let mut i = 0;
    while i < tokens.len() {
        let (kind, text) = tokens[i];

        if !in_decl {
            let is_fn_start = (kind == SyntaxKind::FnKw
                || kind == SyntaxKind::FuncKw
                || kind == SyntaxKind::DefKw)
                || (kind == SyntaxKind::AsyncKw
                    && tokens[i + 1..]
                        .iter()
                        .take(5)
                        .any(|(k, _)| *k == SyntaxKind::FnKw || *k == SyntaxKind::FuncKw || *k == SyntaxKind::DefKw))
                || (kind == SyntaxKind::PubKw
                    || kind == SyntaxKind::PublicKw
                    || kind == SyntaxKind::ExportKw)
                    && tokens[i + 1..].iter().take(5).any(|(k, _)| {
                        *k == SyntaxKind::FnKw
                            || *k == SyntaxKind::FuncKw
                            || *k == SyntaxKind::DefKw
                            || *k == SyntaxKind::AsyncKw
                    });

            let is_type_start = (kind == SyntaxKind::StructKw
                || kind == SyntaxKind::ClassKw
                || kind == SyntaxKind::InterfaceKw
                || kind == SyntaxKind::EnumKw
                || kind == SyntaxKind::TraitKw
                || kind == SyntaxKind::UnionKw
                || kind == SyntaxKind::TypedefKw)
                || (kind == SyntaxKind::PubKw
                    || kind == SyntaxKind::PublicKw
                    || kind == SyntaxKind::ExportKw)
                    && tokens[i + 1..].iter().take(5).any(|(k, _)| {
                        *k == SyntaxKind::StructKw
                            || *k == SyntaxKind::ClassKw
                            || *k == SyntaxKind::InterfaceKw
                            || *k == SyntaxKind::EnumKw
                            || *k == SyntaxKind::TraitKw
                            || *k == SyntaxKind::UnionKw
                    });

            if is_fn_start {
                builder.start_node(SyntaxKind::FunctionDecl);
                in_decl = true;
            } else if is_type_start {
                builder.start_node(SyntaxKind::StructDecl);
                in_decl = true;
            }
        }

        builder.token(kind, text);

        if kind == SyntaxKind::OpenBrace {
            brace_depth += 1;
        } else if kind == SyntaxKind::CloseBrace {
            if brace_depth > 0 {
                brace_depth -= 1;
            }
            if brace_depth == 0 && in_decl {
                builder.finish_node();
                in_decl = false;
            }
        } else if kind == SyntaxKind::Semicolon && brace_depth == 0 && in_decl {
            builder.finish_node();
            in_decl = false;
        }

        i += 1;
    }

    if in_decl {
        builder.finish_node();
    }

    builder.finish_node(); // Root
    Ok(builder.finish())
}

// ---------------------------------------------------------------------------
// 1. Rust CST Driver
// ---------------------------------------------------------------------------

pub use crate::cst::polyglot::systems::RustCstDriver;


// ---------------------------------------------------------------------------
// 2. TypeScript / JavaScript / TSX CST Driver
// ---------------------------------------------------------------------------

static RE_TS_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

static RE_TS_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:export\s+)?(?:abstract\s+)?class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

static RE_TS_INTERFACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:export\s+)?interface\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

static RE_TS_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:export\s+)?type\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

static RE_TS_CONST_COMPONENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:export\s+)?const\s+([A-Z][a-zA-Z0-9_$]*)\s*(?::\s*[^=]+)?\s*=\s*(?:async\s*)?(?:\([^)]*\)|[a-zA-Z_$][a-zA-Z0-9_$]*)\s*=>").unwrap()
});

#[derive(Debug, Default, Clone)]
pub struct TypeScriptCstDriver {
    pub language: LanguageId,
}

impl TypeScriptCstDriver {
    pub fn new(language: LanguageId) -> Self {
        Self { language }
    }
}

impl UniversalCstDriver for TypeScriptCstDriver {
    fn language_id(&self) -> LanguageId {
        self.language.clone()
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, self.language.clone()))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        // Functions
        for cap in RE_TS_FN.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.tsx".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("export"),
                });
            }
        }

        // Classes
        for cap in RE_TS_CLASS.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    file: "source.tsx".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("export"),
                });
            }
        }

        // Interfaces & Types
        for cap in RE_TS_INTERFACE.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Trait,
                    file: "source.tsx".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("export"),
                });
            }
        }

        for cap in RE_TS_TYPE.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::TypeAlias,
                    file: "source.tsx".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("export"),
                });
            }
        }

        // Arrow Component Functions
        for cap in RE_TS_CONST_COMPONENT.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.tsx".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("export"),
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(self.language.clone(), 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        for (kind, text) in tokens {
            if kind == SyntaxKind::IfKw {
                let b_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    b_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if branch",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, b_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(b_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
            } else if kind == SyntaxKind::WhileKw || kind == SyntaxKind::ForKw {
                let l_id = node_counter;
                node_counter += 1;
                let body_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    l_id,
                    CfgNodeKind::LoopHeader,
                    TextRange::at(0, text.len() as TextSize),
                    "loop header",
                ));
                cfg.add_node(CfgNode::new(
                    body_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "loop body",
                ));

                cfg.add_edge(CfgEdge::new(last_node, l_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(l_id, body_id, CfgEdgeKind::TrueBranch));
                cfg.add_edge(CfgEdge::new(body_id, l_id, CfgEdgeKind::LoopBack));
                last_node = l_id;
            } else if kind == SyntaxKind::SwitchKw {
                let sw_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    sw_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "switch statement",
                ));
                cfg.add_edge(CfgEdge::new(last_node, sw_id, CfgEdgeKind::Sequential));
                last_node = sw_id;
            } else if kind == SyntaxKind::ReturnKw {
                let ret_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    ret_id,
                    CfgNodeKind::Return,
                    TextRange::at(0, text.len() as TextSize),
                    "return",
                ));
                cfg.add_edge(CfgEdge::new(last_node, ret_id, CfgEdgeKind::Return));
                last_node = ret_id;
            }
        }

        let exit_id = node_counter;
        cfg.add_node(CfgNode::new(
            exit_id,
            CfgNodeKind::Exit,
            TextRange::at(source.len() as TextSize, 0),
            "exit",
        ));
        cfg.add_edge(CfgEdge::new(last_node, exit_id, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(exit_id);

        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        let report = AstGuard::verify(&source);

        if !report.passed {
            if let Some(violation) = report.violation {
                use crate::types::ViolationKind;
                let bit = match violation {
                    ViolationKind::UnbalancedDelimiters => 0,
                    ViolationKind::AsyncMutexAcrossAwait => 1,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::ArrayOutOfBounds => 3,
                    ViolationKind::UnsafeUnwrap => 4,
                    ViolationKind::ReDoSPattern => 5,
                    ViolationKind::NullDereference => 6,
                    ViolationKind::ConditionalHookCall => 7,
                    ViolationKind::ClientSecretLeak => 8,
                    ViolationKind::UnsafeInnerHtml => 9,
                    ViolationKind::JsxTagMismatch => 10,
                    ViolationKind::SqlInjection => 11,
                    ViolationKind::FloatingPromise => 12,
                    ViolationKind::ReactStateRace => 13,
                    ViolationKind::ListenerLeak => 14,
                    ViolationKind::InsecureRandomness => 15,
                    ViolationKind::PathTraversal => 16,
                    ViolationKind::UnboundedRegex => 17,
                    ViolationKind::DynamicCodeEval => 18,
                    ViolationKind::UntypedUnionAccess => 19,
                    ViolationKind::CircularMemLeak => 20,
                    ViolationKind::AsyncCancellationSafety => 21,
                    ViolationKind::ConstantTimeCrypto => 22,
                    ViolationKind::ExhaustiveEnumNarrowing => 23,
                    ViolationKind::ResourceDescriptorLeak => 24,
                    ViolationKind::SsrfUnsafeFetch => 25,
                    ViolationKind::UnboundedChannelDeadlock => 26,
                    ViolationKind::PrototypePollution => 27,
                    ViolationKind::CorsWildcardCredentials => 28,
                    ViolationKind::HardcodedKeyEntropy => 29,
                    ViolationKind::UncheckedArithmeticOverflow => 30,
                    ViolationKind::AtomicStateMutation => 31,
                };
                bitset.set_violation(bit);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Python CST Driver
// ---------------------------------------------------------------------------

static RE_PY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

static RE_PY_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

#[derive(Debug, Default, Clone)]
pub struct PythonCstDriver;

impl UniversalCstDriver for PythonCstDriver {
    fn language_id(&self) -> LanguageId {
        LanguageId::Python
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, LanguageId::Python))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        for cap in RE_PY_DEF.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_private = name.as_str().starts_with('_');
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.py".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: !is_private,
                });
            }
        }

        for cap in RE_PY_CLASS.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_private = name.as_str().starts_with('_');
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    file: "source.py".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: !is_private,
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(LanguageId::Python, 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        for (kind, text) in tokens {
            if kind == SyntaxKind::IfKw {
                let b_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    b_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if branch",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, b_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(b_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
            } else if kind == SyntaxKind::WhileKw || kind == SyntaxKind::ForKw {
                let l_id = node_counter;
                node_counter += 1;
                let body_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    l_id,
                    CfgNodeKind::LoopHeader,
                    TextRange::at(0, text.len() as TextSize),
                    "loop header",
                ));
                cfg.add_node(CfgNode::new(
                    body_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "loop body",
                ));

                cfg.add_edge(CfgEdge::new(last_node, l_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(l_id, body_id, CfgEdgeKind::TrueBranch));
                cfg.add_edge(CfgEdge::new(body_id, l_id, CfgEdgeKind::LoopBack));
                last_node = l_id;
            } else if kind == SyntaxKind::ReturnKw {
                let ret_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    ret_id,
                    CfgNodeKind::Return,
                    TextRange::at(0, text.len() as TextSize),
                    "return",
                ));
                cfg.add_edge(CfgEdge::new(last_node, ret_id, CfgEdgeKind::Return));
                last_node = ret_id;
            }
        }

        let exit_id = node_counter;
        cfg.add_node(CfgNode::new(
            exit_id,
            CfgNodeKind::Exit,
            TextRange::at(source.len() as TextSize, 0),
            "exit",
        ));
        cfg.add_edge(CfgEdge::new(last_node, exit_id, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(exit_id);

        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        let report = AstGuard::verify(&source);
        if !report.passed {
            if let Some(violation) = report.violation {
                use crate::types::ViolationKind;
                let bit = match violation {
                    ViolationKind::UnbalancedDelimiters => 0,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::ArrayOutOfBounds => 3,
                    ViolationKind::PathTraversal => 16,
                    ViolationKind::DynamicCodeEval => 18,
                    ViolationKind::HardcodedKeyEntropy => 29,
                    _ => 0,
                };
                bitset.set_violation(bit);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Go & Systems CST Drivers
// ---------------------------------------------------------------------------

pub use crate::cst::polyglot::systems::{CCstDriver, CppCstDriver, GoCstDriver};
pub type CCppCstDriver = CppCstDriver;

// ---------------------------------------------------------------------------
// 6. Dart CST Driver
// ---------------------------------------------------------------------------

static RE_DART_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:abstract\s+)?class\s+([a-zA-Z0-9_]+)").unwrap()
});

static RE_DART_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:[a-zA-Z0-9_<>]+\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*(?:async\s*)?\{").unwrap()
});

#[derive(Debug, Default, Clone)]
pub struct DartCstDriver;

impl UniversalCstDriver for DartCstDriver {
    fn language_id(&self) -> LanguageId {
        LanguageId::Dart
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, LanguageId::Dart))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        for cap in RE_DART_CLASS.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_private = name.as_str().starts_with('_');
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    file: "source.dart".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: !is_private,
                });
            }
        }

        for cap in RE_DART_FN.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_private = name.as_str().starts_with('_');
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.dart".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: !is_private,
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(LanguageId::Dart, 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        for (kind, text) in tokens {
            if kind == SyntaxKind::IfKw {
                let b_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    b_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if branch",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, b_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(b_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
            } else if kind == SyntaxKind::WhileKw || kind == SyntaxKind::ForKw {
                let l_id = node_counter;
                node_counter += 1;
                let body_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    l_id,
                    CfgNodeKind::LoopHeader,
                    TextRange::at(0, text.len() as TextSize),
                    "loop header",
                ));
                cfg.add_node(CfgNode::new(
                    body_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "loop body",
                ));

                cfg.add_edge(CfgEdge::new(last_node, l_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(l_id, body_id, CfgEdgeKind::TrueBranch));
                cfg.add_edge(CfgEdge::new(body_id, l_id, CfgEdgeKind::LoopBack));
                last_node = l_id;
            } else if kind == SyntaxKind::ReturnKw {
                let ret_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    ret_id,
                    CfgNodeKind::Return,
                    TextRange::at(0, text.len() as TextSize),
                    "return",
                ));
                cfg.add_edge(CfgEdge::new(last_node, ret_id, CfgEdgeKind::Return));
                last_node = ret_id;
            }
        }

        let exit_id = node_counter;
        cfg.add_node(CfgNode::new(
            exit_id,
            CfgNodeKind::Exit,
            TextRange::at(source.len() as TextSize, 0),
            "exit",
        ));
        cfg.add_edge(CfgEdge::new(last_node, exit_id, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(exit_id);

        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        let report = AstGuard::verify(&source);
        if !report.passed {
            if let Some(violation) = report.violation {
                use crate::types::ViolationKind;
                let bit = match violation {
                    ViolationKind::UnbalancedDelimiters => 0,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::ArrayOutOfBounds => 3,
                    ViolationKind::NullDereference => 6,
                    _ => 0,
                };
                bitset.set_violation(bit);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Java CST Driver
// ---------------------------------------------------------------------------

static RE_JAVA_PACKAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*package\s+([a-zA-Z0-9_.]+)\s*;").unwrap()
});

static RE_JAVA_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:public\s+|protected\s+|private\s+)?(?:abstract\s+|final\s+)?(class|interface|enum|record)\s+([a-zA-Z0-9_]+)").unwrap()
});

static RE_JAVA_METHOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:public\s+|protected\s+|private\s+)?(?:static\s+)?(?:final\s+)?(?:[a-zA-Z0-9_<>]+\s+)+([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*(?:throws\s+[^{]+)?\{").unwrap()
});

#[derive(Debug, Default, Clone)]
pub struct JavaCstDriver;

impl UniversalCstDriver for JavaCstDriver {
    fn language_id(&self) -> LanguageId {
        LanguageId::Java
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, LanguageId::Java))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        for cap in RE_JAVA_PACKAGE.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Module,
                    file: "source.java".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: full.as_str().trim().to_string(),
                    doc_comment: None,
                    is_exported: true,
                });
            }
        }

        for cap in RE_JAVA_CLASS.captures_iter(&source) {
            if let (Some(full), Some(k), Some(name)) = (cap.get(0), cap.get(1), cap.get(2)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_exported = full.as_str().contains("public");
                let sym_kind = match k.as_str() {
                    "interface" => SymbolKind::Trait,
                    "enum" => SymbolKind::Enum,
                    _ => SymbolKind::Struct,
                };
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: sym_kind,
                    file: "source.java".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported,
                });
            }
        }

        for cap in RE_JAVA_METHOD.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_exported = full.as_str().contains("public");
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.java".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported,
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(LanguageId::Java, 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        for (kind, text) in tokens {
            if kind == SyntaxKind::IfKw {
                let b_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    b_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if branch",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, b_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(b_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
            } else if kind == SyntaxKind::WhileKw || kind == SyntaxKind::ForKw {
                let l_id = node_counter;
                node_counter += 1;
                let body_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    l_id,
                    CfgNodeKind::LoopHeader,
                    TextRange::at(0, text.len() as TextSize),
                    "loop header",
                ));
                cfg.add_node(CfgNode::new(
                    body_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "loop body",
                ));

                cfg.add_edge(CfgEdge::new(last_node, l_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(l_id, body_id, CfgEdgeKind::TrueBranch));
                cfg.add_edge(CfgEdge::new(body_id, l_id, CfgEdgeKind::LoopBack));
                last_node = l_id;
            } else if kind == SyntaxKind::ReturnKw {
                let ret_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    ret_id,
                    CfgNodeKind::Return,
                    TextRange::at(0, text.len() as TextSize),
                    "return",
                ));
                cfg.add_edge(CfgEdge::new(last_node, ret_id, CfgEdgeKind::Return));
                last_node = ret_id;
            }
        }

        let exit_id = node_counter;
        cfg.add_node(CfgNode::new(
            exit_id,
            CfgNodeKind::Exit,
            TextRange::at(source.len() as TextSize, 0),
            "exit",
        ));
        cfg.add_edge(CfgEdge::new(last_node, exit_id, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(exit_id);

        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        let report = AstGuard::verify(&source);
        if !report.passed {
            if let Some(violation) = report.violation {
                use crate::types::ViolationKind;
                let bit = match violation {
                    ViolationKind::UnbalancedDelimiters => 0,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::ArrayOutOfBounds => 3,
                    ViolationKind::NullDereference => 6,
                    ViolationKind::ResourceDescriptorLeak => 24,
                    ViolationKind::SsrfUnsafeFetch => 25,
                    _ => 0,
                };
                bitset.set_violation(bit);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. PHP CST Driver
// ---------------------------------------------------------------------------

static RE_PHP_NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*namespace\s+([a-zA-Z0-9_\\]+)\s*;").unwrap()
});

static RE_PHP_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:public\s+|protected\s+|private\s+)?(?:static\s+)?function\s+([a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)").unwrap()
});

static RE_PHP_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:abstract\s+|final\s+)?(class|trait|interface|enum)\s+([a-zA-Z_\x80-\xff][a-zA-Z0-9_\x80-\xff]*)").unwrap()
});

#[derive(Debug, Default, Clone)]
pub struct PhpCstDriver;

impl UniversalCstDriver for PhpCstDriver {
    fn language_id(&self) -> LanguageId {
        LanguageId::Php
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, LanguageId::Php))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        for cap in RE_PHP_NAMESPACE.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Module,
                    file: "source.php".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: full.as_str().trim().to_string(),
                    doc_comment: None,
                    is_exported: true,
                });
            }
        }

        for cap in RE_PHP_CLASS.captures_iter(&source) {
            if let (Some(full), Some(k), Some(name)) = (cap.get(0), cap.get(1), cap.get(2)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let sym_kind = match k.as_str() {
                    "interface" | "trait" => SymbolKind::Trait,
                    "enum" => SymbolKind::Enum,
                    _ => SymbolKind::Struct,
                };
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: sym_kind,
                    file: "source.php".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: true,
                });
            }
        }

        for cap in RE_PHP_FN.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                let is_private = full.as_str().contains("private");
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    file: "source.php".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: !is_private,
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(LanguageId::Php, 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        for (kind, text) in tokens {
            if kind == SyntaxKind::IfKw {
                let b_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    b_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if branch",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, b_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(b_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
            } else if kind == SyntaxKind::WhileKw || kind == SyntaxKind::ForKw {
                let l_id = node_counter;
                node_counter += 1;
                let body_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    l_id,
                    CfgNodeKind::LoopHeader,
                    TextRange::at(0, text.len() as TextSize),
                    "loop header",
                ));
                cfg.add_node(CfgNode::new(
                    body_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "loop body",
                ));

                cfg.add_edge(CfgEdge::new(last_node, l_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(l_id, body_id, CfgEdgeKind::TrueBranch));
                cfg.add_edge(CfgEdge::new(body_id, l_id, CfgEdgeKind::LoopBack));
                last_node = l_id;
            } else if kind == SyntaxKind::ReturnKw {
                let ret_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    ret_id,
                    CfgNodeKind::Return,
                    TextRange::at(0, text.len() as TextSize),
                    "return",
                ));
                cfg.add_edge(CfgEdge::new(last_node, ret_id, CfgEdgeKind::Return));
                last_node = ret_id;
            }
        }

        let exit_id = node_counter;
        cfg.add_node(CfgNode::new(
            exit_id,
            CfgNodeKind::Exit,
            TextRange::at(source.len() as TextSize, 0),
            "exit",
        ));
        cfg.add_edge(CfgEdge::new(last_node, exit_id, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(exit_id);

        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        let report = AstGuard::verify(&source);
        if !report.passed {
            if let Some(violation) = report.violation {
                use crate::types::ViolationKind;
                let bit = match violation {
                    ViolationKind::UnbalancedDelimiters => 0,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::SqlInjection => 11,
                    ViolationKind::PathTraversal => 16,
                    ViolationKind::DynamicCodeEval => 18,
                    _ => 0,
                };
                bitset.set_violation(bit);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 9. Generic Fallback Polyglot Driver
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GenericPolyglotDriver {
    pub language: LanguageId,
}

impl GenericPolyglotDriver {
    pub fn new(language: LanguageId) -> Self {
        Self { language }
    }
}

impl UniversalCstDriver for GenericPolyglotDriver {
    fn language_id(&self) -> LanguageId {
        self.language.clone()
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        let green = build_lossless_green(source)?;
        Ok(GreenTreeRoot::new(green, self.language.clone()))
    }

    fn extract_symbols(&self, _tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        Vec::new()
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(self.language.clone(), 0);
        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        let exit = CfgNode::new(1, CfgNodeKind::Exit, TextRange::at(source.len() as TextSize, 0), "exit");
        cfg.add_node(entry);
        cfg.add_node(exit);
        cfg.add_edge(CfgEdge::new(0, 1, CfgEdgeKind::Sequential));
        cfg.exit_id = Some(1);
        cfg
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        let source = tree.source_lossless_text();
        if !AstGuard::check_delimiter_balance(&source) {
            bitset.set_violation(0);
        }
    }
}
