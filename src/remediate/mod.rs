//! Remediate module.

#![forbid(unsafe_code)]

pub mod auto_fixer;
pub mod patch_strategy;
pub mod index;

pub use index::*;
