//! Green Tree: Immutable, Unparented, Content-Addressed CST Nodes.
//!
//! Stores syntax kind, exact byte lengths, and child elements while retaining 100% of trivia
//! (whitespace, comments, newlines).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// SyntaxKind
// ---------------------------------------------------------------------------

/// Complete enumeration of concrete syntax tokens, trivia, keywords, and composite nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum SyntaxKind {
    // --- Trivia ---
    Whitespace = 0,
    Newline,
    LineComment,
    BlockComment,
    DocComment,

    // --- Keywords ---
    FnKw,
    StructKw,
    EnumKw,
    TraitKw,
    ImplKw,
    TypeKw,
    PubKw,
    AsyncKw,
    LetKw,
    ConstKw,
    MutKw,
    IfKw,
    ElseKw,
    MatchKw,
    ReturnKw,
    UseKw,
    ModKw,
    ImportKw,
    ExportKw,
    FromKw,
    DefaultKw,
    ClassKw,
    InterfaceKw,
    ExtendsKw,
    DefKw,

    // --- Literals ---
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    CharLiteral,
    BoolLiteral,

    // --- Punctuation & Delimiters ---
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Semicolon,
    Colon,
    ColonColon,
    Comma,
    Dot,
    QuestionDot,
    Arrow,
    FatArrow,
    Eq,
    EqEq,
    Excl,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Lt,
    Gt,
    LtEq,
    GtEq,
    BangEq,

    // --- Identifiers & Names ---
    Ident,
    Lifetime,

    // --- JSX Specific ---
    JsxTagOpen,
    JsxTagClose,
    JsxSelfClose,
    JsxText,
    JsxAttribute,

    // --- Composite Structural Nodes ---
    Root,
    FunctionDecl,
    StructDecl,
    EnumDecl,
    TraitDecl,
    ImplBlock,
    Param,
    ParamList,
    Block,
    ReturnExpr,
    CallExpr,
    BinaryExpr,
    FieldAccess,
    VariableDecl,
    UseStmt,
    ImportStmt,
    ExportStmt,
    JsxElement,
    ErrorNode,
}

impl SyntaxKind {
    /// Returns true if this kind represents trivia (whitespace, comments, newlines).
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
                | SyntaxKind::DocComment
        )
    }

    /// Returns true if this kind represents a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            SyntaxKind::FnKw
                | SyntaxKind::StructKw
                | SyntaxKind::EnumKw
                | SyntaxKind::TraitKw
                | SyntaxKind::ImplKw
                | SyntaxKind::TypeKw
                | SyntaxKind::PubKw
                | SyntaxKind::AsyncKw
                | SyntaxKind::LetKw
                | SyntaxKind::ConstKw
                | SyntaxKind::MutKw
                | SyntaxKind::IfKw
                | SyntaxKind::ElseKw
                | SyntaxKind::MatchKw
                | SyntaxKind::ReturnKw
                | SyntaxKind::UseKw
                | SyntaxKind::ModKw
                | SyntaxKind::ImportKw
                | SyntaxKind::ExportKw
                | SyntaxKind::FromKw
                | SyntaxKind::DefaultKw
                | SyntaxKind::ClassKw
                | SyntaxKind::InterfaceKw
                | SyntaxKind::ExtendsKw
                | SyntaxKind::DefKw
        )
    }

    /// Returns true if this kind represents punctuation or delimiter.
    pub fn is_punct(&self) -> bool {
        matches!(
            self,
            SyntaxKind::OpenParen
                | SyntaxKind::CloseParen
                | SyntaxKind::OpenBrace
                | SyntaxKind::CloseBrace
                | SyntaxKind::OpenBracket
                | SyntaxKind::CloseBracket
                | SyntaxKind::Semicolon
                | SyntaxKind::Colon
                | SyntaxKind::ColonColon
                | SyntaxKind::Comma
                | SyntaxKind::Dot
                | SyntaxKind::QuestionDot
                | SyntaxKind::Arrow
                | SyntaxKind::FatArrow
                | SyntaxKind::Eq
                | SyntaxKind::EqEq
                | SyntaxKind::Excl
                | SyntaxKind::Plus
                | SyntaxKind::Minus
                | SyntaxKind::Star
                | SyntaxKind::Slash
                | SyntaxKind::Percent
                | SyntaxKind::Amp
                | SyntaxKind::Pipe
                | SyntaxKind::Lt
                | SyntaxKind::Gt
                | SyntaxKind::LtEq
                | SyntaxKind::GtEq
                | SyntaxKind::BangEq
        )
    }

    /// Returns true if this kind is a composite AST/CST node.
    pub fn is_composite_node(&self) -> bool {
        matches!(
            self,
            SyntaxKind::Root
                | SyntaxKind::FunctionDecl
                | SyntaxKind::StructDecl
                | SyntaxKind::EnumDecl
                | SyntaxKind::TraitDecl
                | SyntaxKind::ImplBlock
                | SyntaxKind::Param
                | SyntaxKind::ParamList
                | SyntaxKind::Block
                | SyntaxKind::ReturnExpr
                | SyntaxKind::CallExpr
                | SyntaxKind::BinaryExpr
                | SyntaxKind::FieldAccess
                | SyntaxKind::VariableDecl
                | SyntaxKind::UseStmt
                | SyntaxKind::ImportStmt
                | SyntaxKind::ExportStmt
                | SyntaxKind::JsxElement
                | SyntaxKind::ErrorNode
        )
    }
}

// ---------------------------------------------------------------------------
// TextSize & TextRange
// ---------------------------------------------------------------------------

pub type TextSize = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    pub fn new(start: TextSize, end: TextSize) -> Self {
        debug_assert!(start <= end, "TextRange start must be <= end");
        Self { start, end }
    }

    pub fn at(offset: TextSize, len: TextSize) -> Self {
        Self::new(offset, offset + len)
    }

    pub fn empty(offset: TextSize) -> Self {
        Self::new(offset, offset)
    }

    pub fn start(&self) -> TextSize {
        self.start
    }

    pub fn end(&self) -> TextSize {
        self.end
    }

    pub fn len(&self) -> TextSize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, offset: TextSize) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn contains_range(&self, other: TextRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn cover(&self, other: TextRange) -> TextRange {
        TextRange::new(self.start.min(other.start), self.end.max(other.end))
    }
}

// ---------------------------------------------------------------------------
// GreenToken
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreenToken {
    kind: SyntaxKind,
    text: Arc<str>,
}

impl GreenToken {
    pub fn new(kind: SyntaxKind, text: &str) -> Self {
        Self {
            kind,
            text: Arc::from(text),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_len(&self) -> TextSize {
        self.text.len() as TextSize
    }
}

impl fmt::Display for GreenToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

// ---------------------------------------------------------------------------
// GreenElement
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GreenElement {
    Node(GreenNode),
    Token(GreenToken),
}

impl GreenElement {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            GreenElement::Node(n) => n.kind(),
            GreenElement::Token(t) => t.kind(),
        }
    }

    pub fn text_len(&self) -> TextSize {
        match self {
            GreenElement::Node(n) => n.text_len(),
            GreenElement::Token(t) => t.text_len(),
        }
    }

    pub fn as_node(&self) -> Option<&GreenNode> {
        match self {
            GreenElement::Node(n) => Some(n),
            GreenElement::Token(_) => None,
        }
    }

    pub fn as_token(&self) -> Option<&GreenToken> {
        match self {
            GreenElement::Token(t) => Some(t),
            GreenElement::Node(_) => None,
        }
    }
}

impl fmt::Display for GreenElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GreenElement::Node(n) => write!(f, "{}", n),
            GreenElement::Token(t) => write!(f, "{}", t),
        }
    }
}

// ---------------------------------------------------------------------------
// GreenNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GreenNodeData {
    kind: SyntaxKind,
    text_len: TextSize,
    children: Vec<GreenElement>,
    content_hash: u64,
}

/// Immutable, unparented, content-addressed syntax node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreenNode {
    data: Arc<GreenNodeData>,
}

impl GreenNode {
    /// Create a new GreenNode from children, computing text length and FNV-1a hash.
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Self {
        let mut text_len = 0;
        let mut hasher_bytes = Vec::new();
        hasher_bytes.extend_from_slice(&(kind as u16).to_le_bytes());

        for child in &children {
            text_len += child.text_len();
            match child {
                GreenElement::Node(n) => {
                    hasher_bytes.extend_from_slice(&n.data.content_hash.to_le_bytes());
                }
                GreenElement::Token(t) => {
                    hasher_bytes.extend_from_slice(t.text().as_bytes());
                }
            }
        }

        let content_hash = fnv1a_64(&hasher_bytes);

        Self {
            data: Arc::new(GreenNodeData {
                kind,
                text_len,
                children,
                content_hash,
            }),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data.kind
    }

    pub fn text_len(&self) -> TextSize {
        self.data.text_len
    }

    pub fn children(&self) -> &[GreenElement] {
        &self.data.children
    }

    pub fn content_hash(&self) -> u64 {
        self.data.content_hash
    }

    pub fn child_count(&self) -> usize {
        self.data.children.len()
    }
}

impl fmt::Display for GreenNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in &self.data.children {
            write!(f, "{}", child)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GreenNodeBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing GreenNode trees iteratively.
#[derive(Default)]
pub struct GreenNodeBuilder {
    stack: Vec<(SyntaxKind, usize)>,
    children: Vec<GreenElement>,
}

impl GreenNodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        let start_idx = self.children.len();
        self.stack.push((kind, start_idx));
    }

    pub fn token(&mut self, kind: SyntaxKind, text: &str) {
        self.children
            .push(GreenElement::Token(GreenToken::new(kind, text)));
    }

    pub fn finish_node(&mut self) {
        let (kind, start_idx) = if let Some(item) = self.stack.pop() {
            item
        } else {
            return;
        };
        let node_children: Vec<GreenElement> = self.children.drain(start_idx..).collect();
        let node = GreenNode::new(kind, node_children);
        self.children.push(GreenElement::Node(node));
    }

    pub fn finish(mut self) -> GreenNode {
        while !self.stack.is_empty() {
            self.finish_node();
        }

        match self.children.pop() {
            Some(GreenElement::Node(n)) => n,
            Some(GreenElement::Token(t)) => {
                GreenNode::new(SyntaxKind::Root, vec![GreenElement::Token(t)])
            }
            None => GreenNode::new(SyntaxKind::Root, Vec::new()),
        }
    }
}
