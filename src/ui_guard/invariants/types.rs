//! UI Layout Invariants, Box Constraints, and Hazard Violation Types.
//!
//! Enforces mathematical layout constraints, RenderFlex bounds, build-phase side-effect guards,
//! and list keying governance across Flutter, React, and TSX component trees.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Layout Violation Kinds
// ---------------------------------------------------------------------------

/// Categories of UI layout invariants and rendering hazard violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutViolationKind {
    /// An unbounded widget (e.g. `ListView`, `GridView`, `Column`) placed directly inside a Flex container without `Expanded` or `Flexible`.
    RenderFlexOverflowUnboundedChild,
    /// A scrollable nested inside another scrollable along the same scroll axis without `shrinkWrap: true` or `NeverScrollableScrollPhysics`.
    NestedScrollableSameAxis,
    /// A state mutation (e.g. `setState()`, `useState` setter, dispatch) invoked directly during the component build/render pass.
    BuildPhaseStateMutation,
    /// An iterative list rendering expression (e.g. `.map()`, `ListView.builder`) lacking a distinct, stable `key` attribute.
    MissingListKey,
    /// Negative margin, padding, or dimensions that collapse layout bounding boxes.
    NegativeConstraintOverflow,
    /// A TSX flex-col container with fixed-height children that overflow without `overflow-y-auto` or `min-h-0`.
    UnboundedFlexHeight,
    /// A text or element contrast ratio failing WCAG accessibility standards.
    ContrastTokenViolation,
}

impl LayoutViolationKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::RenderFlexOverflowUnboundedChild => "RENDERFLEX_OVERFLOW_UNBOUNDED_CHILD",
            Self::NestedScrollableSameAxis => "NESTED_SCROLLABLE_SAME_AXIS",
            Self::BuildPhaseStateMutation => "BUILD_PHASE_STATE_MUTATION",
            Self::MissingListKey => "MISSING_LIST_KEY",
            Self::NegativeConstraintOverflow => "NEGATIVE_CONSTRAINT_OVERFLOW",
            Self::UnboundedFlexHeight => "UNBOUNDED_FLEX_HEIGHT",
            Self::ContrastTokenViolation => "CONTRAST_TOKEN_VIOLATION",
        }
    }
}

impl fmt::Display for LayoutViolationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

// ---------------------------------------------------------------------------
// Box Constraint Bounds
// ---------------------------------------------------------------------------

/// Mathematical 2D Box Constraints modeling rendering min/max bounds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxConstraintBounds {
    pub min_width: f32,
    pub max_width: Option<f32>,
    pub min_height: f32,
    pub max_height: Option<f32>,
    pub is_bounded_vertical: bool,
    pub is_bounded_horizontal: bool,
}

impl Default for BoxConstraintBounds {
    fn default() -> Self {
        Self::unbounded()
    }
}

impl BoxConstraintBounds {
    /// Creates unbounded constraints (both axes infinite).
    pub fn unbounded() -> Self {
        Self {
            min_width: 0.0,
            max_width: None,
            min_height: 0.0,
            max_height: None,
            is_bounded_vertical: false,
            is_bounded_horizontal: false,
        }
    }

    /// Creates tight constraints with exact width and height.
    pub fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: Some(width),
            min_height: height,
            max_height: Some(height),
            is_bounded_vertical: true,
            is_bounded_horizontal: true,
        }
    }

    /// Creates loose constraints with maximum bounds.
    pub fn loose(max_width: f32, max_height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width: Some(max_width),
            min_height: 0.0,
            max_height: Some(max_height),
            is_bounded_vertical: true,
            is_bounded_horizontal: true,
        }
    }

    /// Returns true if height is strictly bounded.
    pub fn has_bounded_height(&self) -> bool {
        self.is_bounded_vertical && self.max_height.is_some()
    }

    /// Returns true if width is strictly bounded.
    pub fn has_bounded_width(&self) -> bool {
        self.is_bounded_horizontal && self.max_width.is_some()
    }
}

// ---------------------------------------------------------------------------
// Layout Violation
// ---------------------------------------------------------------------------

/// A concrete UI invariant violation detected in the component/widget tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutViolation {
    pub kind: LayoutViolationKind,
    pub element_id: String,
    pub line_offset: usize,
    pub description: String,
    pub suggested_patch: String,
}

// ---------------------------------------------------------------------------
// UI Invariant Report
// ---------------------------------------------------------------------------

/// Verification report generated by the UI Invariant Solver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiInvariantReport {
    pub passed: bool,
    pub violations: Vec<LayoutViolation>,
    pub latency_ms: f64,
}

impl UiInvariantReport {
    pub fn clean(latency_ms: f64) -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
            latency_ms,
        }
    }

    pub fn with_violations(violations: Vec<LayoutViolation>, latency_ms: f64) -> Self {
        Self {
            passed: violations.is_empty(),
            violations,
            latency_ms,
        }
    }
}
