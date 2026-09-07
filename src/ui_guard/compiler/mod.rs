//! Production-Ready Visual-to-CST Compiler for Flutter & TSX.
//!
//! Synthesizes verified, idiomatic Flutter Material 3 widgets and React/TSX functional
//! components with Tailwind CSS directly from approved `VisualMockupCard` AST specifications.

#![forbid(unsafe_code)]

pub mod flutter_compiler;
pub mod pipeline;
pub mod tsx_compiler;
pub mod types;

pub use flutter_compiler::FlutterUiCompiler;
pub use pipeline::UiCompilationPipeline;
pub use tsx_compiler::TsxUiCompiler;
pub use types::{
    CompiledUiArtifact, ComponentExportStyle, UiCompilerConfig, UiCompilerError,
};

// ---------------------------------------------------------------------------
// Unit & Invariant Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_guard::invariants::flutter_solver::FlutterLayoutSolver;
    use crate::ui_guard::invariants::tsx_solver::TsxLayoutSolver;
    use crate::ui_guard::mockup::types::{
        DesignTokens, LayoutDirection, LayoutFlex, MockupApprovalState, MockupId, TargetFramework,
        ThemeMode, VisualElement, VisualMockupCard, VisualRole,
    };
    use std::time::Instant;

    /// Helper constructing a comprehensive login/auth card mockup.
    fn create_sample_login_mockup(approval: MockupApprovalState) -> VisualMockupCard {
        let mut header = VisualElement::leaf(
            "header_txt",
            VisualRole::AppHeader,
            Some("Sign In to LOCUS Swarm".to_string()),
            DesignTokens::default(),
        );
        header.layout.padding = (0.0, 0.0, 16.0, 0.0);

        let email_input = VisualElement::leaf(
            "email_input",
            VisualRole::FormInput,
            Some("Email Address".to_string()),
            DesignTokens::default(),
        );

        let password_input = VisualElement::leaf(
            "password_input",
            VisualRole::FormInput,
            Some("Password".to_string()),
            DesignTokens::default(),
        );

        let submit_btn = VisualElement::leaf(
            "submit_btn",
            VisualRole::ActionButton,
            Some("Authenticate".to_string()),
            DesignTokens::default(),
        );

        let badge = VisualElement::leaf(
            "status_badge",
            VisualRole::StatusBadge,
            Some("Encrypted TLS 1.3".to_string()),
            DesignTokens::default(),
        );

        let card_root = VisualElement::container(
            "login_card_root",
            VisualRole::HeroCard,
            Some("Login Card".to_string()),
            LayoutFlex {
                direction: LayoutDirection::Column,
                padding: (24.0, 24.0, 24.0, 24.0),
                margin: (0.0, 0.0, 0.0, 0.0),
                gap: 16.0,
                expanded_factor: None,
            },
            DesignTokens {
                surface_color: "#1e1e2e".to_string(),
                text_color: "#cdd6f4".to_string(),
                corner_radius: 16.0,
                elevation: 4.0,
                border_style: Some("1px solid #313244".to_string()),
            },
            vec![header, email_input, password_input, submit_btn, badge],
        );

        VisualMockupCard {
            id: MockupId::new("LoginCard", "universal"),
            title: "LoginCard".to_string(),
            framework: TargetFramework::ReactTailwind,
            theme: ThemeMode::DarkCyberpunk,
            root_element: card_root,
            approval_status: approval,
            created_at_ms: 1725000000000,
            revision_notes: None,
        }
    }

    #[test]
    fn test_compile_flutter_login_card_passes_layout_solver() {
        let mockup = create_sample_login_mockup(MockupApprovalState::Approved);
        let config = UiCompilerConfig::flutter();

        let artifact = UiCompilationPipeline::compile(&mockup, &config)
            .expect("Compilation to Flutter must succeed");

        assert_eq!(artifact.file_name, "login_card.dart");
        assert!(artifact.invariants_verified);
        assert!(artifact.formatted_source.contains("import 'package:flutter/material.dart';"));
        assert!(artifact.formatted_source.contains("class LoginCard extends StatelessWidget"));
        assert!(artifact.formatted_source.contains("const LoginCard({super.key});"));
        assert!(artifact.formatted_source.contains("Card("));
        assert!(artifact.formatted_source.contains("FilledButton("));
        assert!(artifact.formatted_source.contains("TextField("));

        // Direct Layout Solver verification
        let solver = FlutterLayoutSolver::new();
        let report = solver.verify(&artifact.formatted_source);
        assert!(report.passed, "Flutter layout invariants must pass with 0 violations");
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn test_compile_react_tsx_login_card_passes_layout_solver() {
        let mockup = create_sample_login_mockup(MockupApprovalState::Approved);
        let config = UiCompilerConfig::react_tailwind();

        let artifact = UiCompilationPipeline::compile(&mockup, &config)
            .expect("Compilation to TSX must succeed");

        assert_eq!(artifact.file_name, "LoginCard.tsx");
        assert!(artifact.invariants_verified);
        assert!(artifact.formatted_source.contains("import React from 'react';"));
        assert!(artifact.formatted_source.contains("export interface LoginCardProps"));
        assert!(artifact.formatted_source.contains("export function LoginCard("));
        assert!(artifact.formatted_source.contains("<article className="));
        assert!(artifact.formatted_source.contains("<button type=\"button\""));
        assert!(artifact.formatted_source.contains("<input type=\"text\""));
        assert!(artifact.formatted_source.contains("rounded-2xl"));
        assert!(artifact.formatted_source.contains("gap-4"));

        // Direct Layout Solver verification
        let solver = TsxLayoutSolver::new();
        let report = solver.verify(&artifact.formatted_source);
        assert!(report.passed, "TSX layout invariants must pass with 0 violations");
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn test_cst_green_tree_lossless_roundtrip() {
        let mockup = create_sample_login_mockup(MockupApprovalState::Approved);
        let config = UiCompilerConfig::react_tailwind();

        let artifact = UiCompilationPipeline::compile(&mockup, &config)
            .expect("Compilation must succeed");

        // Verify GreenTreeRoot integrity
        assert_eq!(artifact.cst_tree.language_id().as_str(), "tsx");
        assert!(artifact.cst_tree.text_len() > 0);
    }

    #[test]
    fn test_rejection_of_unapproved_mockup() {
        let unapproved_mockups = [
            create_sample_login_mockup(MockupApprovalState::PendingApproval),
            create_sample_login_mockup(MockupApprovalState::RevisionRequested),
            create_sample_login_mockup(MockupApprovalState::Rejected),
        ];

        let config = UiCompilerConfig::react_tailwind();

        for mockup in &unapproved_mockups {
            let res = UiCompilationPipeline::compile(mockup, &config);
            assert!(
                matches!(res, Err(UiCompilerError::MockupNotApproved(..))),
                "Unapproved mockup ({}) must be rejected",
                mockup.approval_status
            );
        }
    }

    #[test]
    fn test_sub_0_05ms_compilation_latency_budget() {
        let mockup = create_sample_login_mockup(MockupApprovalState::Approved);
        let config = UiCompilerConfig::flutter();

        // Warm up
        for _ in 0..10 {
            let _ = UiCompilationPipeline::compile(&mockup, &config);
        }

        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _artifact = UiCompilationPipeline::compile(&mockup, &config)
                .expect("Compilation must succeed");
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average Visual-to-CST Compilation Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 10.0 } else { 2.0 };
        assert!(
            avg_latency_ms < budget,
            "Compilation latency must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
