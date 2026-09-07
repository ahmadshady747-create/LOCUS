//! Core types for the Lossless Polyglot Green-Red Concrete Syntax Tree (CST) Subsystem.
//!
//! Provides zero-unsafe, zero-heap-waste structures for polyglot language representations,
//! lossless tree roots, compact graph representations, and deterministic invariant bitsets.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cst::green::{GreenElement, GreenNode, TextRange, TextSize};
use crate::cst::red::{SyntaxNode, SyntaxToken};

// ---------------------------------------------------------------------------
// Language Identifier
// ---------------------------------------------------------------------------

/// Complete enumeration of recognized programming and configuration languages.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LanguageId {
    // --- Systems Languages ---
    #[default]
    Rust,
    C,
    Cpp,
    Go,

    // --- Application & Scripting Languages ---
    TypeScript,
    JavaScript,
    Tsx,
    Jsx,
    Dart,
    Python,
    Php,
    Java,

    // --- Modern Web Frameworks ---
    Svelte,
    Astro,
    Vue,

    // --- Fallback Generic ---
    Generic(String),
}

impl LanguageId {
    /// Returns true if this language is a systems-level compiled language.
    pub fn is_systems(&self) -> bool {
        matches!(
            self,
            LanguageId::Rust | LanguageId::C | LanguageId::Cpp | LanguageId::Go
        )
    }

    /// Returns true if this language is an application or scripting language.
    pub fn is_application(&self) -> bool {
        !self.is_systems()
    }

    /// Returns true if this language is a frontend web component or UI language.
    pub fn is_frontend(&self) -> bool {
        matches!(
            self,
            LanguageId::TypeScript
                | LanguageId::JavaScript
                | LanguageId::Tsx
                | LanguageId::Jsx
                | LanguageId::Svelte
                | LanguageId::Astro
                | LanguageId::Vue
                | LanguageId::Dart
        )
    }

    /// Infers the language identifier from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        let clean = ext.trim_start_matches('.').to_lowercase();
        match clean.as_str() {
            "rs" => LanguageId::Rust,
            "c" | "h" => LanguageId::C,
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => LanguageId::Cpp,
            "go" => LanguageId::Go,
            "ts" | "mts" | "cts" => LanguageId::TypeScript,
            "js" | "mjs" | "cjs" => LanguageId::JavaScript,
            "tsx" => LanguageId::Tsx,
            "jsx" => LanguageId::Jsx,
            "dart" => LanguageId::Dart,
            "py" | "pyw" | "pyi" => LanguageId::Python,
            "php" | "phtml" | "php5" | "php7" | "php8" => LanguageId::Php,
            "java" => LanguageId::Java,
            "svelte" => LanguageId::Svelte,
            "astro" => LanguageId::Astro,
            "vue" => LanguageId::Vue,
            other => LanguageId::Generic(other.to_string()),
        }
    }

    /// Canonical string identifier for the language.
    pub fn as_str(&self) -> &str {
        match self {
            LanguageId::Rust => "rust",
            LanguageId::C => "c",
            LanguageId::Cpp => "cpp",
            LanguageId::Go => "go",
            LanguageId::TypeScript => "typescript",
            LanguageId::JavaScript => "javascript",
            LanguageId::Tsx => "tsx",
            LanguageId::Jsx => "jsx",
            LanguageId::Dart => "dart",
            LanguageId::Python => "python",
            LanguageId::Php => "php",
            LanguageId::Java => "java",
            LanguageId::Svelte => "svelte",
            LanguageId::Astro => "astro",
            LanguageId::Vue => "vue",
            LanguageId::Generic(s) => s.as_str(),
        }
    }

    /// Primary standard file extensions associated with the language.
    pub fn standard_extensions(&self) -> &'static [&'static str] {
        match self {
            LanguageId::Rust => &["rs"],
            LanguageId::C => &["c", "h"],
            LanguageId::Cpp => &["cpp", "hpp", "cc", "cxx", "hxx"],
            LanguageId::Go => &["go"],
            LanguageId::TypeScript => &["ts", "mts", "cts"],
            LanguageId::JavaScript => &["js", "mjs", "cjs"],
            LanguageId::Tsx => &["tsx"],
            LanguageId::Jsx => &["jsx"],
            LanguageId::Dart => &["dart"],
            LanguageId::Python => &["py", "pyi"],
            LanguageId::Php => &["php"],
            LanguageId::Java => &["java"],
            LanguageId::Svelte => &["svelte"],
            LanguageId::Astro => &["astro"],
            LanguageId::Vue => &["vue"],
            LanguageId::Generic(_) => &[],
        }
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for LanguageId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_lowercase();
        let lang = match clean.as_str() {
            "rust" | "rs" => LanguageId::Rust,
            "c" => LanguageId::C,
            "cpp" | "c++" | "cxx" => LanguageId::Cpp,
            "go" | "golang" => LanguageId::Go,
            "typescript" | "ts" => LanguageId::TypeScript,
            "javascript" | "js" => LanguageId::JavaScript,
            "tsx" | "react-tsx" => LanguageId::Tsx,
            "jsx" | "react-jsx" => LanguageId::Jsx,
            "dart" | "flutter" => LanguageId::Dart,
            "python" | "py" => LanguageId::Python,
            "php" => LanguageId::Php,
            "java" => LanguageId::Java,
            "svelte" => LanguageId::Svelte,
            "astro" => LanguageId::Astro,
            "vue" => LanguageId::Vue,
            other => LanguageId::Generic(other.to_string()),
        };
        Ok(lang)
    }
}

// ---------------------------------------------------------------------------
// Tree Statistics
// ---------------------------------------------------------------------------

/// Structural metrics of a parsed Lossless Concrete Syntax Tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeStats {
    pub node_count: usize,
    pub token_count: usize,
    pub trivia_count: usize,
    pub max_depth: usize,
    pub total_bytes: usize,
}

// ---------------------------------------------------------------------------
// Lossless CST Root Container
// ---------------------------------------------------------------------------

/// Immutable, content-addressed root container for a language-specific Green Tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenTreeRoot {
    green: GreenNode,
    language_id: LanguageId,
    text_len: TextSize,
    content_hash: u64,
}

impl GreenTreeRoot {
    /// Wraps a root `GreenNode` with language metadata.
    pub fn new(green: GreenNode, language_id: LanguageId) -> Self {
        let text_len = green.text_len();
        let content_hash = green.content_hash();
        Self {
            green,
            language_id,
            text_len,
            content_hash,
        }
    }

    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    pub fn language_id(&self) -> &LanguageId {
        &self.language_id
    }

    pub fn text_len(&self) -> TextSize {
        self.text_len
    }

    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// Reconstructs the exact, 100% byte-for-byte source text from the lossless tree.
    pub fn source_lossless_text(&self) -> String {
        self.green.to_string()
    }

    /// Constructs a navigable, parent-linked `SyntaxNode` red tree root.
    pub fn to_syntax_root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Traverses and computes structural metrics for the CST.
    pub fn stats(&self) -> TreeStats {
        let mut node_count = 0;
        let mut token_count = 0;
        let mut trivia_count = 0;
        let mut max_depth = 0;

        fn visit(
            elem: &GreenElement,
            depth: usize,
            n_count: &mut usize,
            t_count: &mut usize,
            tr_count: &mut usize,
            m_depth: &mut usize,
        ) {
            *m_depth = (*m_depth).max(depth);
            match elem {
                GreenElement::Node(n) => {
                    *n_count += 1;
                    for child in n.children() {
                        visit(child, depth + 1, n_count, t_count, tr_count, m_depth);
                    }
                }
                GreenElement::Token(t) => {
                    *t_count += 1;
                    if t.kind().is_trivia() {
                        *tr_count += 1;
                    }
                }
            }
        }

        let root_elem = GreenElement::Node(self.green.clone());
        visit(
            &root_elem,
            1,
            &mut node_count,
            &mut token_count,
            &mut trivia_count,
            &mut max_depth,
        );

        TreeStats {
            node_count,
            token_count,
            trivia_count,
            max_depth,
            total_bytes: self.text_len as usize,
        }
    }

    /// Finds the leaf token located at the given byte offset.
    pub fn find_token_at_offset(&self, offset: TextSize) -> Option<SyntaxToken> {
        let root = self.to_syntax_root();
        root.token_at_offset(offset)
    }

    /// Finds the tightest `SyntaxNode` spanning the given `TextRange`.
    pub fn find_node_at_range(&self, range: TextRange) -> Option<SyntaxNode> {
        let root = self.to_syntax_root();
        root.find_node_at_range(range)
    }
}

impl fmt::Display for GreenTreeRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.green)
    }
}

// ---------------------------------------------------------------------------
// CST Parse Error Definitions
// ---------------------------------------------------------------------------

/// Typed error representation for CST parser and driver operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CstParseError {
    #[error("Unbalanced delimiter: expected '{expected}', found {found:?} at offset {offset}")]
    UnbalancedDelimiter {
        expected: char,
        found: Option<char>,
        offset: TextSize,
    },

    #[error("Invalid token '{token}' at offset {offset}: {message}")]
    InvalidToken {
        token: String,
        offset: TextSize,
        message: String,
    },

    #[error("Unexpected end of file, expected '{expected}'")]
    UnexpectedEof { expected: String },

    #[error("Language invariant violation '{rule}' at offset {offset}: {message}")]
    SyntaxViolation {
        rule: String,
        offset: TextSize,
        message: String,
    },

    #[error("No CST driver registered for language '{0}'")]
    DriverNotFound(String),

    #[error("Internal CST error: {0}")]
    InternalError(String),
}

// ---------------------------------------------------------------------------
// Control Flow Graph (CFG) Definitions
// ---------------------------------------------------------------------------

/// Syntactic and execution role of a basic block or statement in the CFG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CfgNodeKind {
    Entry,
    Exit,
    BasicBlock,
    BranchCondition,
    Branch,
    LoopHeader,
    LoopExit,
    Jump,
    Return,
    MatchArm,
    SwitchCase,
    ExceptionCatch,
    Defer,
    GoroutineSpawn,
}

impl fmt::Display for CfgNodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CfgNodeKind::Entry => "ENTRY",
            CfgNodeKind::Exit => "EXIT",
            CfgNodeKind::BasicBlock => "BLOCK",
            CfgNodeKind::BranchCondition | CfgNodeKind::Branch => "BRANCH",
            CfgNodeKind::LoopHeader => "LOOP_HDR",
            CfgNodeKind::LoopExit => "LOOP_EXIT",
            CfgNodeKind::Jump => "JUMP",
            CfgNodeKind::Return => "RETURN",
            CfgNodeKind::MatchArm => "MATCH_ARM",
            CfgNodeKind::SwitchCase => "SWITCH_CASE",
            CfgNodeKind::ExceptionCatch => "CATCH",
            CfgNodeKind::Defer => "DEFER",
            CfgNodeKind::GoroutineSpawn => "GO_SPAWN",
        };
        write!(f, "{}", s)
    }
}

/// A discrete node in the Control Flow Graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfgNode {
    pub id: u32,
    pub kind: CfgNodeKind,
    pub range: TextRange,
    pub label: String,
}

impl CfgNode {
    pub fn new(id: u32, kind: CfgNodeKind, range: TextRange, label: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            range,
            label: label.into(),
        }
    }
}

/// Transition edge type between control flow graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CfgEdgeKind {
    Sequential,
    TrueBranch,
    FalseBranch,
    ConditionalTrue,
    ConditionalFalse,
    LoopBack,
    LoopBreak,
    LoopContinue,
    Exception,
    MatchCase(String),
    Return,
    DirectJump,
}

impl fmt::Display for CfgEdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CfgEdgeKind::Sequential => write!(f, "seq"),
            CfgEdgeKind::TrueBranch | CfgEdgeKind::ConditionalTrue => write!(f, "true"),
            CfgEdgeKind::FalseBranch | CfgEdgeKind::ConditionalFalse => write!(f, "false"),
            CfgEdgeKind::LoopBack => write!(f, "loop_back"),
            CfgEdgeKind::LoopBreak => write!(f, "break"),
            CfgEdgeKind::LoopContinue => write!(f, "continue"),
            CfgEdgeKind::Exception => write!(f, "catch"),
            CfgEdgeKind::MatchCase(c) => write!(f, "case({})", c),
            CfgEdgeKind::Return => write!(f, "return"),
            CfgEdgeKind::DirectJump => write!(f, "jump"),
        }
    }
}

/// A directed edge connecting two CFG nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfgEdge {
    pub from: u32,
    pub to: u32,
    pub kind: CfgEdgeKind,
}

impl CfgEdge {
    pub fn new(from: u32, to: u32, kind: CfgEdgeKind) -> Self {
        Self { from, to, kind }
    }
}

/// Deterministic, language-agnostic Control Flow Graph representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFlowGraph {
    pub language_id: LanguageId,
    pub nodes: Vec<CfgNode>,
    pub edges: Vec<CfgEdge>,
    pub entry_id: u32,
    pub exit_id: Option<u32>,
}

impl ControlFlowGraph {
    /// Creates a new CFG initialized with an entry node.
    pub fn new(language_id: LanguageId, entry_id: u32) -> Self {
        Self {
            language_id,
            nodes: Vec::new(),
            edges: Vec::new(),
            entry_id,
            exit_id: None,
        }
    }

    /// Appends a new node to the CFG and returns its ID.
    pub fn add_node(&mut self, node: CfgNode) -> u32 {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Connects two nodes with a directed edge.
    pub fn add_edge(&mut self, edge: CfgEdge) {
        self.edges.push(edge);
    }

    /// Computes the McCabe Cyclomatic Complexity: `M = E - N + 2P`.
    /// For single-function connected graphs (P=1), `M = |Edges| - |Nodes| + 2`.
    /// Ensures minimum complexity is 1.
    pub fn cyclomatic_complexity(&self) -> usize {
        let num_nodes = self.nodes.len();
        let num_edges = self.edges.len();
        if num_nodes == 0 {
            return 1;
        }

        // Count decision predicate nodes as alternative exact metric
        let predicate_nodes = self
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    CfgNodeKind::BranchCondition
                        | CfgNodeKind::LoopHeader
                        | CfgNodeKind::MatchArm
                        | CfgNodeKind::SwitchCase
                        | CfgNodeKind::ExceptionCatch
                )
            })
            .count();

        let edge_node_metric = if num_edges >= num_nodes {
            num_edges - num_nodes + 2
        } else {
            1
        };

        (predicate_nodes + 1).max(edge_node_metric)
    }

    /// Identifies all nodes reachable from the entry node using BFS.
    pub fn reachable_nodes(&self) -> Vec<u32> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(self.entry_id);
        visited.insert(self.entry_id);

        while let Some(curr) = queue.pop_front() {
            for edge in &self.edges {
                if edge.from == curr && !visited.contains(&edge.to) {
                    visited.insert(edge.to);
                    queue.push_back(edge.to);
                }
            }
        }

        let mut res: Vec<u32> = visited.into_iter().collect();
        res.sort_unstable();
        res
    }

    /// Identifies dead/unreachable nodes not visited from entry.
    pub fn unreachable_nodes(&self) -> Vec<u32> {
        let reachable = self.reachable_nodes();
        let reachable_set: std::collections::HashSet<u32> = reachable.into_iter().collect();

        self.nodes
            .iter()
            .map(|n| n.id)
            .filter(|id| !reachable_set.contains(id))
            .collect()
    }

    /// Returns the total number of nodes in the CFG.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the CFG contains unreachable (dead) code blocks.
    pub fn has_dead_code(&self) -> bool {
        !self.unreachable_nodes().is_empty()
    }
}

// ---------------------------------------------------------------------------
// Invariant Bitset Definitions (64-bit Compact Bitset)
// ---------------------------------------------------------------------------

/// Trait for types that can index into an `InvariantBitset`.
pub trait IntoBitIndex {
    fn into_bit_index(self) -> usize;
}

impl IntoBitIndex for u8 { fn into_bit_index(self) -> usize { self as usize } }
impl IntoBitIndex for u16 { fn into_bit_index(self) -> usize { self as usize } }
impl IntoBitIndex for u32 { fn into_bit_index(self) -> usize { self as usize } }
impl IntoBitIndex for u64 { fn into_bit_index(self) -> usize { self as usize } }
impl IntoBitIndex for usize { fn into_bit_index(self) -> usize { self } }
impl IntoBitIndex for i32 { fn into_bit_index(self) -> usize { self.max(0) as usize } }
impl IntoBitIndex for i64 { fn into_bit_index(self) -> usize { self.max(0) as usize } }

/// Compact 64-bit bitset representing detected invariant violations or active safety rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InvariantBitset(pub u64);

impl InvariantBitset {
    pub const CLEAN: Self = Self(0);
    pub const ALL_ACTIVE: Self = Self(u64::MAX);

    pub fn new() -> Self {
        Self::CLEAN
    }

    pub fn clean() -> Self {
        Self::CLEAN
    }

    pub fn all() -> Self {
        Self::ALL_ACTIVE
    }

    /// Marks the specified invariant rule as violated (sets bit).
    pub fn set_violation<T: IntoBitIndex>(&mut self, bit: T) {
        let b = bit.into_bit_index();
        if b < 64 {
            self.0 |= 1u64 << b;
        }
    }

    /// Clears the specified invariant violation (unsets bit).
    pub fn clear_violation<T: IntoBitIndex>(&mut self, bit: T) {
        let b = bit.into_bit_index();
        if b < 64 {
            self.0 &= !(1u64 << b);
        }
    }

    /// Checks if a specific invariant rule was violated.
    pub fn has_violation<T: IntoBitIndex>(&self, bit: T) -> bool {
        let b = bit.into_bit_index();
        if b < 64 {
            (self.0 & (1u64 << b)) != 0
        } else {
            false
        }
    }

    /// Returns true if zero invariant violations were detected.
    pub fn is_clean(&self) -> bool {
        self.0 == 0
    }

    /// Returns the total number of recorded invariant violations.
    pub fn violation_count(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns a list of all violated invariant bit indices.
    pub fn violation_bits(&self) -> Vec<u8> {
        let mut bits = Vec::new();
        for i in 0..64 {
            if self.has_violation(i) {
                bits.push(i);
            }
        }
        bits
    }

    /// Merges another invariant bitset into this one via bitwise OR.
    pub fn merge(&mut self, other: InvariantBitset) {
        self.0 |= other.0;
    }
}
