//! locus-engine: Deterministic AST Verification, Semantic Symbol Graph & Surgical Patching Engine.

pub mod cache;
pub mod contract;
pub mod cst;
pub mod diff;
pub mod graph;
pub mod guard;
pub mod lease;
pub mod mcp;
pub mod parser;
pub mod remediate;
pub mod search;
pub mod slice;
pub mod taint;
pub mod tx;
pub mod types;
pub mod wasm;

// Primary public exports
pub use cache::{AstContextCache, CachedEntry};
pub use contract::{ContractSynthesizer, ContractVerificationReport, IntentContract};
pub use cst::{
    parse_to_cst, parse_to_green, to_lossless_text, GreenElement, GreenNode, GreenNodeBuilder,
    GreenToken, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize,
};
pub use diff::{AstDiffEngine, DiffError};
pub use graph::SymbolGraph;
pub use guard::{AstGuard, InvariantsExtended, RuleMask, RuleRunner};
pub use lease::{LeaseBroker, LeaseRegistry};
pub use mcp::run_stdio_server;
pub use parser::{AstNode, AstQueryEngine, IncrementalParser, ParseDelta};
pub use remediate::{AutoFixer, PatchStrategy};
pub use search::{HnswIndex, HybridMatcher};
pub use slice::{ContextSlicer, IntentSlice};
pub use taint::{DataFlowTracker, NullPropagationTracker};
pub use tx::{ShadowBuffer, WorkspaceTransaction};
pub use types::*;
pub use wasm::LocusWasmBridge;
