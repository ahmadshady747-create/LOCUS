//! Pre-Gen Visual UI Mockup Engine & Interactive Preview Cards Barrel.
//!
//! Exposes layout synthesizers, design tokens, dual JSON/ASCII render exporters,
//! and interactive user approval workflows for Flutter and React/Web.

#![forbid(unsafe_code)]

pub mod layout_synthesizer;
pub mod render_exporter;
pub mod types;

pub use layout_synthesizer::VisualLayoutSynthesizer;
pub use render_exporter::MockupRenderExporter;
pub use types::{
    DesignTokens, LayoutDirection, LayoutFlex, MockupApprovalState, MockupError, MockupId,
    TargetFramework, ThemeMode, VisualElement, VisualMockupCard, VisualRole,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_login_screen_synthesis_flutter_and_react() {
        let flutter_card = VisualLayoutSynthesizer::synthesize_login_screen(
            TargetFramework::Flutter,
            ThemeMode::DarkCyberpunk,
        );
        assert_eq!(flutter_card.title, "Login Screen");
        assert_eq!(flutter_card.framework, TargetFramework::Flutter);
        assert_eq!(flutter_card.approval_status, MockupApprovalState::PendingApproval);
        assert!(flutter_card.root_element.total_nodes() >= 5);

        let react_card = VisualLayoutSynthesizer::synthesize_login_screen(
            TargetFramework::ReactTailwind,
            ThemeMode::LightMinimal,
        );
        assert_eq!(react_card.framework, TargetFramework::ReactTailwind);
        assert_eq!(react_card.theme, ThemeMode::LightMinimal);
        assert!(react_card.root_element.total_nodes() >= 5);
    }

    #[test]
    fn test_dashboard_screen_synthesis_and_hierarchy() {
        let dash_card = VisualLayoutSynthesizer::synthesize_dashboard_screen(
            TargetFramework::ReactTailwind,
            ThemeMode::AdaptiveSystem,
        );
        assert_eq!(dash_card.title, "Swarm OS Dashboard");
        assert!(dash_card.root_element.total_nodes() >= 8);
        assert!(VisualLayoutSynthesizer::validate_layout_integrity(&dash_card).is_ok());
    }

    #[test]
    fn test_dual_export_json_and_ascii_art() {
        let card = VisualLayoutSynthesizer::synthesize_login_screen(
            TargetFramework::Flutter,
            ThemeMode::DarkCyberpunk,
        );

        // 1. JSON Schema export
        let json_schema = MockupRenderExporter::to_json_schema(&card).expect("JSON export succeeds");
        assert!(json_schema.contains("login_card_container"));
        assert!(json_schema.contains("input_email"));
        assert!(json_schema.contains("DarkCyberpunk"));

        // 2. Terminal ASCII Art export
        let ascii_art = MockupRenderExporter::to_terminal_ascii_art(&card);
        assert!(ascii_art.contains("LOCUS v2.0 VISUAL PRE-GEN MOCKUP HUD"));
        assert!(ascii_art.contains("Flutter (Material 3 / Cupertino)"));
        assert!(ascii_art.contains("[ ⌨️ Input: Email Address"));
        assert!(ascii_art.contains("[ ▶ Button: Authenticate & Enter"));
    }

    #[test]
    fn test_approval_lifecycle_transitions() {
        let mut card = VisualLayoutSynthesizer::synthesize_login_screen(
            TargetFramework::Flutter,
            ThemeMode::DarkCyberpunk,
        );
        assert_eq!(card.approval_status, MockupApprovalState::PendingApproval);

        // Request Revision
        assert!(MockupRenderExporter::request_revision(&mut card, "Add OAuth Github Button").is_ok());
        assert_eq!(card.approval_status, MockupApprovalState::RevisionRequested);
        assert_eq!(card.revision_notes.as_deref(), Some("Add OAuth Github Button"));

        // Approve Mockup
        assert!(MockupRenderExporter::approve_mockup(&mut card).is_ok());
        assert_eq!(card.approval_status, MockupApprovalState::Approved);
        assert!(card.is_approved());
    }

    #[test]
    fn test_sub_0_05ms_layout_synthesis_budget() {
        let start = Instant::now();
        for _ in 0..100 {
            let card = VisualLayoutSynthesizer::synthesize_dashboard_screen(
                TargetFramework::Flutter,
                ThemeMode::DarkCyberpunk,
            );
            let _ = VisualLayoutSynthesizer::validate_layout_integrity(&card);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 100.0;

        let limit = if cfg!(debug_assertions) { 1000.0 } else { 50.0 };
        assert!(
            avg_us < limit,
            "Layout synthesis and validation must satisfy sub-0.05ms (50µs in release) budget, got: {:.2}µs",
            avg_us
        );
    }
}
