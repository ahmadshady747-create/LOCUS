//! Semantic Invariant Rules S1..S5 Barrel Module.

#![forbid(unsafe_code)]

pub mod async_iterator;
pub mod calendar_arithmetic;
pub mod collection_key;
pub mod date_coercion;
pub mod numeric_sort;

pub use async_iterator::AsyncIteratorRule;
pub use calendar_arithmetic::CalendarArithmeticRule;
pub use collection_key::CollectionKeyRule;
pub use date_coercion::DateCoercionRule;
pub use numeric_sort::NumericSortRule;

use crate::semantic::traits::SemanticRule;

/// Returns instances of all five canonical LOCUS Semantic Invariant Rules (S1..S5).
pub fn all_semantic_rules() -> Vec<Box<dyn SemanticRule>> {
    vec![
        Box::new(AsyncIteratorRule),
        Box::new(CollectionKeyRule),
        Box::new(NumericSortRule),
        Box::new(DateCoercionRule),
        Box::new(CalendarArithmeticRule),
    ]
}
