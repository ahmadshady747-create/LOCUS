//! Cross-File Taint & Type Flow Tracking module.

#![forbid(unsafe_code)]

pub mod data_flow;
pub mod null_analyzer;
pub mod index;

pub use index::*;
