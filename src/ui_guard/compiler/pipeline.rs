//! Unified UI Compilation Pipeline.
//!
//! Orchestrates approval validation, target dispatch, layout constraint verification,
//! and lossless Concrete Syntax Tree synthesis across Flutter and React/TSX ecosystems.

#![forbid(unsafe_code)]

use crate::ui_guard::compiler::flutter_compiler::FlutterUiCompiler;
use crate::ui_guard::compiler::tsx_compiler::TsxUiCompiler;
use crate::ui_guard::compiler::types::{CompiledUiArtifact, UiCompilerConfig, UiCompilerError};
use crate::ui_guard::mockup::types::{TargetFramework, VisualMockupCard};

/// Orchestration pipeline managing multi-platform UI compilation and safety verification.
pub struct UiCompilationPipeline;

impl UiCompilationPipeline {
    /// Compiles an approved visual mockup card into a verified, production-ready UI artifact.
    pub fn compile(
        mockup: &VisualMockupCard,
        config: &UiCompilerConfig,
    ) -> Result<CompiledUiArtifact, UiCompilerError> {
        // 1. Strict Approval Gate
        if !mockup.is_approved() {
            return Err(UiCompilerError::MockupNotApproved(mockup.approval_status));
        }

        // 2. Dispatch to framework-specific compiler
        match config.target_framework {
            TargetFramework::Flutter => {
                let compiler = FlutterUiCompiler::new();
                compiler.compile(mockup, config)
            }
            TargetFramework::ReactTailwind => {
                let compiler = TsxUiCompiler::new();
                compiler.compile(mockup, config)
            }
            TargetFramework::Vue | TargetFramework::Svelte => {
                Err(UiCompilerError::UnsupportedFramework(config.target_framework))
            }
        }
    }
}
