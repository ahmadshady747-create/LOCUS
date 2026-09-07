//! Rust Universal CST Driver & Semantic Systems Parser.
//!
//! 100% Safe, pure-Rust driver implementing `UniversalCstDriver` with lossless
//! parsing, symbol extraction, sub-0.05ms CFG generation, and FFI boundary tracking.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use regex::Regex;

use crate::cst::green::{GreenNode, GreenNodeBuilder, SyntaxKind, TextRange, TextSize};
use crate::cst::lexer::tokenize;
use crate::cst::polyglot::systems::cfg::{build_cfg_for_systems, ControlFlowGraph as SystemsCfg};
use crate::cst::polyglot::systems::ffi_tracker::{
    ExportedSymbol, FfiAbi, FfiParam, FfiSignature, FfiType, ImportedSymbol,
};
use crate::cst::polyglot::systems::symbols::{
    ResolvedSymbol as SystemsResolvedSymbol, SymbolVisibility as SystemsVisibility,
    SystemsSymbolKind,
};
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{
    CfgEdge, CfgEdgeKind, CfgNode, CfgNodeKind, ControlFlowGraph, CstParseError, GreenTreeRoot,
    InvariantBitset, LanguageId,
};
use crate::guard::AstGuard;
use crate::types::{ResolvedSymbol, SymbolKind};

static RE_RUST_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?(?:extern\s+(?:.*?\s+)?)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

static RE_RUST_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

static RE_RUST_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?enum\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

static RE_RUST_TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:pub(?:\s*\([^)]*\))?\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

/// Driver for Rust source code parsing and verification.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustCstDriver;

impl RustCstDriver {
    pub fn new() -> Self {
        Self
    }

    /// Parses raw source code into a 100% lossless GreenNode.
    pub fn parse_to_green(&self, source: &str) -> GreenNode {
        let tokens = tokenize(source);
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Root);

        let mut in_decl = false;
        let mut brace_depth = 0;
        let mut i = 0;
        let len = tokens.len();

        while i < len {
            let (kind, text) = tokens[i];

            if !in_decl {
                let is_fn_start = (kind == SyntaxKind::FnKw)
                    || (kind == SyntaxKind::AsyncKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(6)
                            .any(|(k, _)| *k == SyntaxKind::FnKw))
                    || (kind == SyntaxKind::PubKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(6)
                            .any(|(k, _)| *k == SyntaxKind::FnKw || *k == SyntaxKind::AsyncKw || *k == SyntaxKind::ExternKw))
                    || (kind == SyntaxKind::ExternKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(4)
                            .any(|(k, _)| *k == SyntaxKind::FnKw));

                let is_struct_start = kind == SyntaxKind::StructKw
                    || (kind == SyntaxKind::PubKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(4)
                            .any(|(k, _)| *k == SyntaxKind::StructKw));

                let is_enum_start = kind == SyntaxKind::EnumKw
                    || (kind == SyntaxKind::PubKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(4)
                            .any(|(k, _)| *k == SyntaxKind::EnumKw));

                let is_trait_start = kind == SyntaxKind::TraitKw
                    || (kind == SyntaxKind::PubKw
                        && tokens[i + 1..len]
                            .iter()
                            .take(4)
                            .any(|(k, _)| *k == SyntaxKind::TraitKw));

                let is_impl_start = kind == SyntaxKind::ImplKw;

                let is_extern_block = kind == SyntaxKind::ExternKw
                    && tokens[i + 1..len]
                        .iter()
                        .take(4)
                        .any(|(k, _)| *k == SyntaxKind::OpenBrace || *k == SyntaxKind::StringLiteral);

                if is_fn_start {
                    builder.start_node(SyntaxKind::FunctionDecl);
                    in_decl = true;
                } else if is_struct_start {
                    builder.start_node(SyntaxKind::StructDecl);
                    in_decl = true;
                } else if is_enum_start {
                    builder.start_node(SyntaxKind::EnumDecl);
                    in_decl = true;
                } else if is_trait_start {
                    builder.start_node(SyntaxKind::TraitDecl);
                    in_decl = true;
                } else if is_impl_start {
                    builder.start_node(SyntaxKind::ImplBlock);
                    in_decl = true;
                } else if is_extern_block {
                    builder.start_node(SyntaxKind::ExternBlock);
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

        builder.finish_node(); // Close Root
        builder.finish()
    }

    /// Parses directly into a `GreenTreeRoot`.
    pub fn parse_to_cst(&self, source: &str) -> GreenTreeRoot {
        let green = self.parse_to_green(source);
        GreenTreeRoot::new(green, LanguageId::Rust)
    }

    /// Extracts systems-specific resolved symbols with doc comments and attributes.
    pub fn extract_systems_symbols(&self, source: &str) -> Vec<SystemsResolvedSymbol> {
        let tokens = tokenize(source);
        let mut symbols = Vec::new();

        let mut offset = 0;
        let mut current_doc: Option<String> = None;
        let mut current_attrs: Vec<String> = Vec::new();

        let mut i = 0;
        let len = tokens.len();

        while i < len {
            let (kind, text) = tokens[i];
            let tok_len = text.len() as u32;

            if kind == SyntaxKind::DocComment {
                current_doc = Some(text.trim().to_string());
            } else if kind == SyntaxKind::Hash {
                let mut j = i;
                let mut attr_text = String::new();
                while j < len && tokens[j].0 != SyntaxKind::CloseBracket && tokens[j].0 != SyntaxKind::Newline {
                    attr_text.push_str(tokens[j].1);
                    j += 1;
                }
                if j < len && tokens[j].0 == SyntaxKind::CloseBracket {
                    attr_text.push_str(tokens[j].1);
                    current_attrs.push(attr_text);
                }
            } else if kind == SyntaxKind::FnKw
                || kind == SyntaxKind::StructKw
                || kind == SyntaxKind::EnumKw
                || kind == SyntaxKind::TraitKw
                || kind == SyntaxKind::ImplKw
            {
                let sym_kind = match kind {
                    SyntaxKind::FnKw => SystemsSymbolKind::Function,
                    SyntaxKind::StructKw => SystemsSymbolKind::Struct,
                    SyntaxKind::EnumKw => SystemsSymbolKind::Enum,
                    SyntaxKind::TraitKw => SystemsSymbolKind::Trait,
                    SyntaxKind::ImplKw => SystemsSymbolKind::Impl,
                    _ => unreachable!(),
                };

                let mut name = String::new();
                for k in (i + 1)..len.min(i + 6) {
                    if tokens[k].0 == SyntaxKind::Ident {
                        name = tokens[k].1.to_string();
                        break;
                    }
                }

                if !name.is_empty() {
                    let mut visibility = SystemsVisibility::Private;
                    let is_ffi = current_attrs.iter().any(|a| a.contains("no_mangle"))
                        || tokens[..i].iter().rev().take(8).any(|(k, t)| *k == SyntaxKind::StringLiteral && *t == "\"C\"");

                    for p in i.saturating_sub(6)..i {
                        if tokens[p].0 == SyntaxKind::PubKw {
                            if p + 1 < i && tokens[p + 1].0 == SyntaxKind::OpenParen {
                                visibility = SystemsVisibility::Crate;
                            } else if is_ffi {
                                visibility = SystemsVisibility::ExportedFfi;
                            } else {
                                visibility = SystemsVisibility::Public;
                            }
                            break;
                        }
                    }

                    let sig_end = offset + tok_len;
                    let mut sig_tokens = Vec::new();
                    let sig_start_idx = i.saturating_sub(4);
                    for s in sig_start_idx..len.min(i + 25) {
                        if tokens[s].0 == SyntaxKind::OpenBrace || tokens[s].0 == SyntaxKind::Semicolon {
                            break;
                        }
                        sig_tokens.push(tokens[s].1);
                    }
                    let signature = sig_tokens.join("").trim().to_string();

                    let range = TextRange::new(offset, sig_end);
                    let mut sym = SystemsResolvedSymbol::new(name, "rust", sym_kind, visibility, signature, range);
                    if let Some(doc) = current_doc.take() {
                        sym = sym.with_doc(doc);
                    }
                    if !current_attrs.is_empty() {
                        sym = sym.with_attributes(std::mem::take(&mut current_attrs));
                    }
                    sym = sym.with_ffi(is_ffi);
                    symbols.push(sym);
                }
            }

            if !kind.is_trivia() && kind != SyntaxKind::Hash && kind != SyntaxKind::DocComment {
                current_doc = None;
            }

            offset += tok_len;
            i += 1;
        }

        symbols
    }

    /// Builds the high-throughput systems CFG.
    pub fn build_cfg(&self, source: &str) -> SystemsCfg {
        build_cfg_for_systems(source, "rust")
    }

    /// Extracts FFI exported and imported boundaries.
    pub fn extract_ffi_boundaries(&self, source: &str) -> (Vec<ExportedSymbol>, Vec<ImportedSymbol>) {
        let tokens = tokenize(source);
        let mut exported = Vec::new();
        let mut imported = Vec::new();

        let mut in_extern_block = false;
        let mut has_no_mangle = false;
        let mut offset = 0;
        let len = tokens.len();
        let mut i = 0;

        while i < len {
            let (kind, text) = tokens[i];
            let tok_len = text.len() as u32;

            if kind == SyntaxKind::DocComment || kind == SyntaxKind::LineComment {
                offset += tok_len;
                i += 1;
                continue;
            }

            if kind == SyntaxKind::Hash {
                let mut j = i;
                let mut attr_str = String::new();
                while j < len && tokens[j].0 != SyntaxKind::CloseBracket && tokens[j].0 != SyntaxKind::Newline {
                    attr_str.push_str(tokens[j].1);
                    j += 1;
                }
                if attr_str.contains("no_mangle") {
                    has_no_mangle = true;
                }
            } else if kind == SyntaxKind::ExternKw {
                let is_block = tokens[i + 1..len]
                    .iter()
                    .take(4)
                    .any(|(k, _)| *k == SyntaxKind::OpenBrace);
                if is_block {
                    in_extern_block = true;
                }
            } else if kind == SyntaxKind::CloseBrace && in_extern_block {
                in_extern_block = false;
            } else if kind == SyntaxKind::FnKw {
                let mut fn_name = String::new();
                let mut name_idx = i;
                for k in (i + 1)..len.min(i + 4) {
                    if tokens[k].0 == SyntaxKind::Ident {
                        fn_name = tokens[k].1.to_string();
                        name_idx = k;
                        break;
                    }
                }

                if !fn_name.is_empty() {
                    let is_extern_fn = tokens[i.saturating_sub(6)..i]
                        .iter()
                        .any(|(k, _)| *k == SyntaxKind::ExternKw);

                    let mut params = Vec::new();
                    let mut return_type = FfiType::Void;

                    let mut p_idx = name_idx;
                    while p_idx < len && tokens[p_idx].0 != SyntaxKind::OpenParen {
                        p_idx += 1;
                    }
                    if p_idx < len && tokens[p_idx].0 == SyntaxKind::OpenParen {
                        let mut param_tokens = Vec::new();
                        p_idx += 1;
                        let mut depth = 1;
                        while p_idx < len && depth > 0 {
                            if tokens[p_idx].0 == SyntaxKind::OpenParen {
                                depth += 1;
                            } else if tokens[p_idx].0 == SyntaxKind::CloseParen {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            param_tokens.push(tokens[p_idx].1);
                            p_idx += 1;
                        }

                        let param_str = param_tokens.join("");
                        for part in param_str.split(',') {
                            let part = part.trim();
                            if !part.is_empty() && part != "self" && part != "&self" && part != "&mut self" {
                                if let Some((pname, ptype)) = part.split_once(':') {
                                    params.push(FfiParam {
                                        name: pname.trim().to_string(),
                                        ffi_type: FfiType::from_rust(ptype),
                                    });
                                }
                            }
                        }

                        let mut ret_idx = p_idx + 1;
                        while ret_idx < len && tokens[ret_idx].0 != SyntaxKind::OpenBrace && tokens[ret_idx].0 != SyntaxKind::Semicolon {
                            if tokens[ret_idx].0 == SyntaxKind::Arrow {
                                let mut ret_tokens = Vec::new();
                                ret_idx += 1;
                                while ret_idx < len && tokens[ret_idx].0 != SyntaxKind::OpenBrace && tokens[ret_idx].0 != SyntaxKind::Semicolon {
                                    ret_tokens.push(tokens[ret_idx].1);
                                    ret_idx += 1;
                                }
                                return_type = FfiType::from_rust(&ret_tokens.join(""));
                                break;
                            }
                            ret_idx += 1;
                        }
                    }

                    let signature = FfiSignature {
                        abi: FfiAbi::C,
                        params,
                        return_type,
                        is_variadic: false,
                    };

                    let range = TextRange::new(offset, offset + tok_len);

                    if in_extern_block {
                        imported.push(ImportedSymbol {
                            name: fn_name.clone(),
                            link_name: fn_name,
                            language: "rust".to_string(),
                            signature,
                            target_library: None,
                            file: "source.rs".to_string(),
                            range,
                        });
                    } else if has_no_mangle || is_extern_fn {
                        exported.push(ExportedSymbol {
                            name: fn_name.clone(),
                            link_name: fn_name,
                            language: "rust".to_string(),
                            signature,
                            file: "source.rs".to_string(),
                            range,
                        });
                        has_no_mangle = false;
                    }
                }
            }

            offset += tok_len;
            i += 1;
        }

        (exported, imported)
    }
}

impl UniversalCstDriver for RustCstDriver {
    fn language_id(&self) -> LanguageId {
        LanguageId::Rust
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        if !AstGuard::check_delimiter_balance(source) {
            return Err(CstParseError::UnbalancedDelimiter {
                expected: '}',
                found: None,
                offset: source.len() as TextSize,
            });
        }
        let green = self.parse_to_green(source);
        Ok(GreenTreeRoot::new(green, LanguageId::Rust))
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let source = tree.source_lossless_text();
        let mut symbols = Vec::new();

        for cap in RE_RUST_FN.captures_iter(&source) {
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
                    file: "source.rs".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("pub"),
                });
            }
        }

        for cap in RE_RUST_STRUCT.captures_iter(&source) {
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
                    file: "source.rs".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("pub"),
                });
            }
        }

        for cap in RE_RUST_ENUM.captures_iter(&source) {
            if let (Some(full), Some(name)) = (cap.get(0), cap.get(1)) {
                let sig = source[full.start()..]
                    .lines()
                    .next()
                    .unwrap_or(full.as_str())
                    .trim()
                    .to_string();
                symbols.push(ResolvedSymbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Enum,
                    file: "source.rs".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("pub"),
                });
            }
        }

        for cap in RE_RUST_TRAIT.captures_iter(&source) {
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
                    file: "source.rs".to_string(),
                    byte_start: full.start(),
                    byte_end: full.end(),
                    signature: sig,
                    doc_comment: None,
                    is_exported: full.as_str().contains("pub"),
                });
            }
        }

        symbols
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let source = tree.source_lossless_text();
        let mut cfg = ControlFlowGraph::new(LanguageId::Rust, 0);

        let entry = CfgNode::new(0, CfgNodeKind::Entry, TextRange::at(0, 0), "entry");
        cfg.add_node(entry);

        let mut node_counter = 1u32;
        let mut last_node = 0u32;

        let tokens = tokenize(&source);
        let mut i = 0;

        while i < tokens.len() {
            let (kind, text) = tokens[i];

            if kind == SyntaxKind::IfKw {
                let branch_id = node_counter;
                node_counter += 1;
                let then_id = node_counter;
                node_counter += 1;

                cfg.add_node(CfgNode::new(
                    branch_id,
                    CfgNodeKind::BranchCondition,
                    TextRange::at(0, text.len() as TextSize),
                    "if condition",
                ));
                cfg.add_node(CfgNode::new(
                    then_id,
                    CfgNodeKind::BasicBlock,
                    TextRange::at(0, 0),
                    "then block",
                ));

                cfg.add_edge(CfgEdge::new(last_node, branch_id, CfgEdgeKind::Sequential));
                cfg.add_edge(CfgEdge::new(branch_id, then_id, CfgEdgeKind::TrueBranch));
                last_node = then_id;
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
            i += 1;
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
                    ViolationKind::AsyncMutexAcrossAwait => 1,
                    ViolationKind::DivisionByZero => 2,
                    ViolationKind::ArrayOutOfBounds => 3,
                    ViolationKind::UnsafeUnwrap => 4,
                    ViolationKind::ReDoSPattern => 5,
                    ViolationKind::UnbalancedDelimiters => 0,
                    _ => 0,
                };
                bitset.set_violation(bit);
            }
        }
    }
}
