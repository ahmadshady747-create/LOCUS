//! Red Tree: Navigable, Parent-Aware, Absolute-Offset Syntax Wrappers.
//!
//! Provides transparent hierarchical navigation (parent, children, siblings, descendants,
//! text ranges) around immutable GreenNodes in 100% safe Rust.

#![forbid(unsafe_code)]

use std::fmt;
use std::sync::Arc;

use crate::cst::green::{GreenElement, GreenNode, GreenToken, SyntaxKind, TextRange, TextSize};

// ---------------------------------------------------------------------------
// SyntaxNodeData & SyntaxNode
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SyntaxNodeData {
    parent: Option<SyntaxNode>,
    green: GreenNode,
    offset: TextSize,
    index_in_parent: usize,
}

/// A lightweight, navigable CST node wrapper over a `GreenNode`.
#[derive(Debug, Clone)]
pub struct SyntaxNode {
    data: Arc<SyntaxNodeData>,
}

impl PartialEq for SyntaxNode {
    fn eq(&self, other: &Self) -> bool {
        self.data.offset == other.data.offset
            && self.data.green.content_hash() == other.data.green.content_hash()
            && self.data.green.kind() == other.data.green.kind()
    }
}

impl Eq for SyntaxNode {}

impl SyntaxNode {
    /// Constructs a root `SyntaxNode` with offset 0 and no parent.
    pub fn new_root(green: GreenNode) -> Self {
        Self {
            data: Arc::new(SyntaxNodeData {
                parent: None,
                green,
                offset: 0,
                index_in_parent: 0,
            }),
        }
    }

    /// Constructs a child `SyntaxNode` referencing its parent.
    pub fn new_child(
        parent: SyntaxNode,
        green: GreenNode,
        offset: TextSize,
        index_in_parent: usize,
    ) -> Self {
        Self {
            data: Arc::new(SyntaxNodeData {
                parent: Some(parent),
                green,
                offset,
                index_in_parent,
            }),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data.green.kind()
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::at(self.data.offset, self.data.green.text_len())
    }

    pub fn parent(&self) -> Option<SyntaxNode> {
        self.data.parent.clone()
    }

    pub fn green(&self) -> &GreenNode {
        &self.data.green
    }

    pub fn offset(&self) -> TextSize {
        self.data.offset
    }

    pub fn index_in_parent(&self) -> usize {
        self.data.index_in_parent
    }

    pub fn text(&self) -> String {
        self.data.green.to_string()
    }

    /// Returns direct child nodes (excluding standalone tokens).
    pub fn children(&self) -> Vec<SyntaxNode> {
        let mut result = Vec::new();
        let mut curr_offset = self.data.offset;

        for (idx, child) in self.data.green.children().iter().enumerate() {
            match child {
                GreenElement::Node(n) => {
                    result.push(SyntaxNode::new_child(
                        self.clone(),
                        n.clone(),
                        curr_offset,
                        idx,
                    ));
                    curr_offset += n.text_len();
                }
                GreenElement::Token(t) => {
                    curr_offset += t.text_len();
                }
            }
        }

        result
    }

    /// Returns all direct child elements (both nodes and tokens).
    pub fn children_with_tokens(&self) -> Vec<SyntaxElement> {
        let mut result = Vec::new();
        let mut curr_offset = self.data.offset;

        for (idx, child) in self.data.green.children().iter().enumerate() {
            match child {
                GreenElement::Node(n) => {
                    let node = SyntaxNode::new_child(self.clone(), n.clone(), curr_offset, idx);
                    curr_offset += n.text_len();
                    result.push(SyntaxElement::Node(node));
                }
                GreenElement::Token(t) => {
                    let tok = SyntaxToken::new(self.clone(), t.clone(), curr_offset, idx);
                    curr_offset += t.text_len();
                    result.push(SyntaxElement::Token(tok));
                }
            }
        }

        result
    }

    pub fn first_child(&self) -> Option<SyntaxNode> {
        self.children().into_iter().next()
    }

    pub fn last_child(&self) -> Option<SyntaxNode> {
        self.children().into_iter().next_back()
    }

    pub fn next_sibling(&self) -> Option<SyntaxNode> {
        let parent = self.parent()?;
        let siblings = parent.children();
        let pos = siblings.iter().position(|s| s == self)?;
        siblings.into_iter().nth(pos + 1)
    }

    pub fn prev_sibling(&self) -> Option<SyntaxNode> {
        let parent = self.parent()?;
        let siblings = parent.children();
        let pos = siblings.iter().position(|s| s == self)?;
        if pos > 0 {
            siblings.into_iter().nth(pos - 1)
        } else {
            None
        }
    }

    /// Recursively traverses all descendant syntax nodes in pre-order.
    pub fn descendants(&self) -> Vec<SyntaxNode> {
        let mut list = Vec::new();
        for child in self.children() {
            list.push(child.clone());
            list.extend(child.descendants());
        }
        list
    }

    /// Recursively traverses all descendant syntax elements (nodes & tokens) in pre-order.
    pub fn descendants_with_tokens(&self) -> Vec<SyntaxElement> {
        let mut list = Vec::new();
        for child in self.children_with_tokens() {
            match child {
                SyntaxElement::Node(n) => {
                    list.push(SyntaxElement::Node(n.clone()));
                    list.extend(n.descendants_with_tokens());
                }
                SyntaxElement::Token(t) => {
                    list.push(SyntaxElement::Token(t));
                }
            }
        }
        list
    }

    /// Finds the leaf token containing the specified byte offset.
    pub fn token_at_offset(&self, offset: TextSize) -> Option<SyntaxToken> {
        for elem in self.descendants_with_tokens() {
            if let SyntaxElement::Token(tok) = elem {
                if tok.text_range().contains(offset) {
                    return Some(tok);
                }
            }
        }
        None
    }

    /// Finds the tightest SyntaxNode whose range encloses `range`.
    pub fn find_node_at_range(&self, range: TextRange) -> Option<SyntaxNode> {
        if !self.text_range().contains_range(range) {
            return None;
        }

        for child in self.children() {
            if child.text_range().contains_range(range) {
                if let Some(inner) = child.find_node_at_range(range) {
                    return Some(inner);
                }
                return Some(child);
            }
        }

        Some(self.clone())
    }
}

impl fmt::Display for SyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data.green)
    }
}

// ---------------------------------------------------------------------------
// SyntaxTokenData & SyntaxToken
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SyntaxTokenData {
    parent: SyntaxNode,
    green: GreenToken,
    offset: TextSize,
    index_in_parent: usize,
}

/// A leaf token wrapper holding parent linkage and absolute byte span.
#[derive(Debug, Clone)]
pub struct SyntaxToken {
    data: Arc<SyntaxTokenData>,
}

impl PartialEq for SyntaxToken {
    fn eq(&self, other: &Self) -> bool {
        self.data.offset == other.data.offset
            && self.data.green == other.data.green
            && self.data.index_in_parent == other.data.index_in_parent
    }
}

impl Eq for SyntaxToken {}

impl SyntaxToken {
    pub fn new(
        parent: SyntaxNode,
        green: GreenToken,
        offset: TextSize,
        index_in_parent: usize,
    ) -> Self {
        Self {
            data: Arc::new(SyntaxTokenData {
                parent,
                green,
                offset,
                index_in_parent,
            }),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data.green.kind()
    }

    pub fn text(&self) -> &str {
        self.data.green.text()
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::at(self.data.offset, self.data.green.text_len())
    }

    pub fn parent(&self) -> SyntaxNode {
        self.data.parent.clone()
    }

    pub fn green(&self) -> &GreenToken {
        &self.data.green
    }

    pub fn offset(&self) -> TextSize {
        self.data.offset
    }

    pub fn index_in_parent(&self) -> usize {
        self.data.index_in_parent
    }
}

impl fmt::Display for SyntaxToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data.green)
    }
}

// ---------------------------------------------------------------------------
// SyntaxElement
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}

impl SyntaxElement {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            SyntaxElement::Node(n) => n.kind(),
            SyntaxElement::Token(t) => t.kind(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            SyntaxElement::Node(n) => n.text_range(),
            SyntaxElement::Token(t) => t.text_range(),
        }
    }

    pub fn as_node(&self) -> Option<&SyntaxNode> {
        match self {
            SyntaxElement::Node(n) => Some(n),
            SyntaxElement::Token(_) => None,
        }
    }

    pub fn as_token(&self) -> Option<&SyntaxToken> {
        match self {
            SyntaxElement::Token(t) => Some(t),
            SyntaxElement::Node(_) => None,
        }
    }

    pub fn parent(&self) -> Option<SyntaxNode> {
        match self {
            SyntaxElement::Node(n) => n.parent(),
            SyntaxElement::Token(t) => Some(t.parent()),
        }
    }
}

impl fmt::Display for SyntaxElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxElement::Node(n) => write!(f, "{}", n),
            SyntaxElement::Token(t) => write!(f, "{}", t),
        }
    }
}
