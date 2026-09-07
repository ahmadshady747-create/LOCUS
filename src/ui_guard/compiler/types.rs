//! Types and configuration models for the Visual-to-CST Compiler.
//!
//! Defines compiler configuration, export styles, compiled UI artifacts,
//! and error domain models for Flutter and React/TSX code generation.

#![forbid(unsafe_code)]

use crate::cst::polyglot::types::GreenTreeRoot;
use crate::ui_guard::invariants::types::{LayoutViolation, UiInvariantReport};
use crate::ui_guard::mockup::types::{MockupApprovalState, TargetFramework};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Export Styles
// ---------------------------------------------------------------------------

/// Export style for synthesized UI components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentExportStyle {
    /// Named export (e.g. `export function MyCard(...)` or class `MyCard`).
    NamedExport,
    /// Default export (e.g. `export default function MyCard(...)`).
    DefaultExport,
}

impl Default for ComponentExportStyle {
    fn default() -> Self {
        Self::NamedExport
    }
}

// ---------------------------------------------------------------------------
// Compiler Configuration
// ---------------------------------------------------------------------------

/// Configuration options for the Visual-to-CST compiler pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiCompilerConfig {
    /// Target framework (Flutter or ReactTailwind).
    pub target_framework: TargetFramework,
    /// Explicit component name (e.g. `LoginCard`). Inferred if None.
    pub component_name: Option<String>,
    /// Whether to generate semantic documentation comments.
    pub generate_comments: bool,
    /// Optimize with `const` constructors in Flutter.
    pub use_const_constructors: bool,
    /// Enforce strict null checks in TypeScript interfaces.
    pub typescript_strict_nulls: bool,
    /// Component export convention.
    pub export_style: ComponentExportStyle,
}

impl Default for UiCompilerConfig {
    fn default() -> Self {
        Self {
            target_framework: TargetFramework::ReactTailwind,
            component_name: None,
            generate_comments: true,
            use_const_constructors: true,
            typescript_strict_nulls: true,
            export_style: ComponentExportStyle::NamedExport,
        }
    }
}

impl UiCompilerConfig {
    /// Preset configuration for Flutter Material 3 code synthesis.
    pub fn flutter() -> Self {
        Self {
            target_framework: TargetFramework::Flutter,
            component_name: None,
            generate_comments: true,
            use_const_constructors: true,
            typescript_strict_nulls: true,
            export_style: ComponentExportStyle::NamedExport,
        }
    }

    /// Preset configuration for React + Tailwind TSX code synthesis.
    pub fn react_tailwind() -> Self {
        Self {
            target_framework: TargetFramework::ReactTailwind,
            component_name: None,
            generate_comments: true,
            use_const_constructors: false,
            typescript_strict_nulls: true,
            export_style: ComponentExportStyle::NamedExport,
        }
    }
}

// ---------------------------------------------------------------------------
// Compiled UI Artifact
// ---------------------------------------------------------------------------

/// Result of a successful Visual-to-CST compilation.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledUiArtifact {
    /// Target output file name (e.g. `login_card.dart` or `LoginCard.tsx`).
    pub file_name: String,
    /// Lossless Green Concrete Syntax Tree root.
    pub cst_tree: GreenTreeRoot,
    /// Formatted, production-ready source code.
    pub formatted_source: String,
    /// Confirmation that all layout and lifecycle invariants passed.
    pub invariants_verified: bool,
    /// Wall-clock compilation latency in milliseconds.
    pub latency_ms: f64,
    /// Detailed verification report from the layout solver.
    pub invariant_report: UiInvariantReport,
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

/// Domain errors during visual mockup compilation.
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiCompilerError {
    #[error("Mockup is not approved for code generation. Current status: {0}")]
    MockupNotApproved(MockupApprovalState),

    #[error("Layout invariant violations detected during verification: {0:?}")]
    LayoutViolationDetected(Vec<LayoutViolation>),

    #[error("Failed to synthesize lossless CST tree: {0}")]
    CstSynthesisFailed(String),

    #[error("Framework '{0}' is not supported by this compiler")]
    UnsupportedFramework(TargetFramework),

    #[error("Internal UI compiler error: {0}")]
    InternalError(String),
}
