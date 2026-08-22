//! locus-engine: Deterministic AST Verification, Semantic Symbol Graph & Surgical Patching Engine.

pub mod cache;
pub mod contract;
pub mod diff;
pub mod graph;
pub mod guard;
pub mod mcp;
pub mod slice;
pub mod types;

// Primary public exports
pub use cache::{AstContextCache, CachedEntry};
pub use contract::{ContractSynthesizer, ContractVerificationReport, IntentContract};
pub use diff::{AstDiffEngine, DiffError};
pub use graph::SymbolGraph;
pub use guard::AstGuard;
pub use mcp::run_stdio_server;
pub use slice::{ContextSlicer, IntentSlice};
pub use types::*;
