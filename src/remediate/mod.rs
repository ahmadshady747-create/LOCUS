//! Remediate module.

#![forbid(unsafe_code)]

pub mod auto_fixer;
pub mod index;
pub mod patch_strategy;

pub use index::*;
