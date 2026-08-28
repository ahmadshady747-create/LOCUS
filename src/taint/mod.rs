//! Cross-File Taint & Type Flow Tracking module (v2 Inter-Procedural SSA Engine).

#![forbid(unsafe_code)]

pub mod data_flow;
pub mod index;
pub mod null_analyzer;

pub use data_flow::{CallGraph, DataFlowTracker, EdgeKind, NodeKind, TaintEdge, TaintNode};
pub use null_analyzer::NullPropagationTracker;
