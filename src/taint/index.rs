//! Taint module index barrel.

#![forbid(unsafe_code)]

pub use super::data_flow::{CallGraph, DataFlowTracker, EdgeKind, NodeKind, TaintEdge, TaintNode};
pub use super::null_analyzer::NullPropagationTracker;
