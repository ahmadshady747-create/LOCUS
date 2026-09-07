//! Deterministic Symbol Extraction for Systems Languages (Rust, C, C++, Go).
//!
//! Provides content-addressed symbol identification, language-specific semantic classification,
//! visibility scoping, and exact byte span mapping.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cst::green::TextRange;
use crate::types::fnv1a_64;

/// Syntactic category of an extracted symbol across systems languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemsSymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Union,
    Trait,
    Interface,
    Impl,
    TypeAlias,
    Typedef,
    Namespace,
    Template,
    Package,
    Constant,
    Variable,
    Macro,
    IncludeDirective,
    ExternBlock,
}

impl fmt::Display for SystemsSymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SystemsSymbolKind::Function => "function",
            SystemsSymbolKind::Method => "method",
            SystemsSymbolKind::Class => "class",
            SystemsSymbolKind::Struct => "struct",
            SystemsSymbolKind::Enum => "enum",
            SystemsSymbolKind::Union => "union",
            SystemsSymbolKind::Trait => "trait",
            SystemsSymbolKind::Interface => "interface",
            SystemsSymbolKind::Impl => "impl",
            SystemsSymbolKind::TypeAlias => "type",
            SystemsSymbolKind::Typedef => "typedef",
            SystemsSymbolKind::Namespace => "namespace",
            SystemsSymbolKind::Template => "template",
            SystemsSymbolKind::Package => "package",
            SystemsSymbolKind::Constant => "const",
            SystemsSymbolKind::Variable => "var",
            SystemsSymbolKind::Macro => "macro",
            SystemsSymbolKind::IncludeDirective => "include",
            SystemsSymbolKind::ExternBlock => "extern",
        };
        write!(f, "{}", s)
    }
}

/// Visibility and linkage scope of a declared symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolVisibility {
    Public,
    Private,
    Protected,
    Crate,
    Package,
    Internal,
    ExportedFfi,
}

impl fmt::Display for SymbolVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SymbolVisibility::Public => "pub",
            SymbolVisibility::Private => "private",
            SymbolVisibility::Protected => "protected",
            SymbolVisibility::Crate => "pub(crate)",
            SymbolVisibility::Package => "package",
            SymbolVisibility::Internal => "internal",
            SymbolVisibility::ExportedFfi => "exported_ffi",
        };
        write!(f, "{}", s)
    }
}

/// Fully resolved symbol extracted from CST with 100% precision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedSymbol {
    /// Deterministic 64-bit content-addressed hash ID.
    pub id: u64,
    /// Canonical symbol identifier name.
    pub name: String,
    /// Language where this symbol is defined.
    pub language: String,
    /// Syntactic category.
    pub kind: SystemsSymbolKind,
    /// Visibility level.
    pub visibility: SymbolVisibility,
    /// Full declaration signature string.
    pub signature: String,
    /// Exact byte range in source code.
    pub range: TextRange,
    /// Associated doc comment if present.
    pub doc_comment: Option<String>,
    /// Associated attributes / pragmas / annotations.
    pub attributes: Vec<String>,
    /// True if this symbol crosses language boundaries via FFI.
    pub is_ffi_boundary: bool,
}

impl ResolvedSymbol {
    /// Creates a new resolved symbol with a stable content-addressed ID.
    pub fn new(
        name: impl Into<String>,
        language: impl Into<String>,
        kind: SystemsSymbolKind,
        visibility: SymbolVisibility,
        signature: impl Into<String>,
        range: TextRange,
    ) -> Self {
        let name_str = name.into();
        let lang_str = language.into();
        let sig_str = signature.into();
        let hash_input = format!("{}:{}:{:?}:{}", lang_str, name_str, kind, range.start());
        let id = fnv1a_64(hash_input.as_bytes());

        Self {
            id,
            name: name_str,
            language: lang_str,
            kind,
            visibility,
            signature: sig_str,
            range,
            doc_comment: None,
            attributes: Vec::new(),
            is_ffi_boundary: false,
        }
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc_comment = Some(doc.into());
        self
    }

    pub fn with_attributes(mut self, attrs: Vec<String>) -> Self {
        self.attributes = attrs;
        self
    }

    pub fn with_ffi(mut self, is_ffi: bool) -> Self {
        self.is_ffi_boundary = is_ffi;
        self
    }
}
