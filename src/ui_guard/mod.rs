//! UI Guard — Pre-Gen Visual Mockup Engine, Responsive Layout Synthesis & Production-Ready Visual-to-CST Compiler.

#![forbid(unsafe_code)]

pub mod compiler;
pub mod invariants;
pub mod mockup;

pub use compiler::{
    CompiledUiArtifact, ComponentExportStyle, FlutterUiCompiler, TsxUiCompiler, UiCompilationPipeline,
    UiCompilerConfig, UiCompilerError,
};
pub use invariants::{
    BoxConstraintBounds, FlutterLayoutSolver, LayoutViolation, LayoutViolationKind,
    TsxLayoutSolver, UiInvariantChecker, UiInvariantReport,
};
pub use mockup::{
    DesignTokens, LayoutDirection, LayoutFlex, MockupApprovalState, MockupError, MockupId,
    MockupRenderExporter, TargetFramework, ThemeMode, VisualElement, VisualLayoutSynthesizer,
    VisualMockupCard, VisualRole,
};
