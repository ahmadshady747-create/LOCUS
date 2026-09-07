//! Render Exporter and Interactive Preview Cards for Visual Mockups.
//!
//! Exposes dual exports: JSON-Schema payload for GUI/Webviews and Terminal ASCII Art HUD
//! with approval lifecycle state management.

#![forbid(unsafe_code)]

use crate::ui_guard::mockup::types::{
    MockupApprovalState, MockupError, VisualElement, VisualMockupCard, VisualRole,
};

pub struct MockupRenderExporter;

impl MockupRenderExporter {
    /// Exports the visual mockup card as a formatted JSON-Schema payload.
    pub fn to_json_schema(card: &VisualMockupCard) -> Result<String, MockupError> {
        serde_json::to_string_pretty(card).map_err(|e| MockupError::SerializationError(e.to_string()))
    }

    /// Renders the visual mockup card as an eye-catching Terminal ASCII Art preview card.
    pub fn to_terminal_ascii_art(card: &VisualMockupCard) -> String {
        let width = 64;
        let mut out = String::with_capacity(2048);

        // Header Top Frame
        out.push_str("┌────────────────────────────────────────────────────────────────┐\n");
        out.push_str(&format!(
            "│  LOCUS v2.0 VISUAL PRE-GEN MOCKUP HUD                          │\n"
        ));
        out.push_str("├────────────────────────────────────────────────────────────────┤\n");

        // Card Metadata
        let title_line = format!("Title: {}", card.title);
        let frame_line = format!("Target: {}", card.framework);
        let theme_line = format!("Theme: {} | Status: {}", card.theme, card.approval_status);

        out.push_str(&format!("│  {:<62}│\n", title_line));
        out.push_str(&format!("│  {:<62}│\n", frame_line));
        out.push_str(&format!("│  {:<62}│\n", theme_line));
        out.push_str("├────────────────────────────────────────────────────────────────┤\n");

        // Body Content Rendering
        Self::render_element_ascii(&card.root_element, &mut out, 0, width);

        // Footer Frame
        out.push_str("├────────────────────────────────────────────────────────────────┤\n");
        let id_str = format!("Mockup ID: {}", card.id);
        out.push_str(&format!("│  {:<62}│\n", id_str));
        out.push_str("└────────────────────────────────────────────────────────────────┘\n");

        out
    }

    fn render_element_ascii(elem: &VisualElement, out: &mut String, indent_level: usize, max_w: usize) {
        let pad = "  ".repeat(indent_level);
        let content_w = max_w.saturating_sub(6 + indent_level * 2);

        let label = elem.label.as_deref().unwrap_or("Untitled");

        match elem.role {
            VisualRole::AppHeader => {
                let text = format!("╭── [AppHeader] {} ──╮", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::NavigationBar => {
                let text = format!("🧭 [Navigation Bar] {}", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::HeroCard => {
                let text = format!("📦 [Container: {}] (Direction: {:?})", label, elem.layout.direction);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::FormInput => {
                let text = format!("  [ ⌨️ Input: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::ActionButton => {
                let text = format!("  [ ▶ Button: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::StatusBadge => {
                let text = format!("  [ 🛡️ Badge: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::ChartWidget => {
                let text = format!("  [ 📊 Chart: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::DataList => {
                let text = format!("  [ ≡ DataList: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
            VisualRole::ModalSheet => {
                let text = format!("  [ 🗔 ModalSheet: {} ]", label);
                out.push_str(&format!("│  {}{:<w$}│\n", pad, text, w = content_w));
            }
        }

        for child in &elem.children {
            Self::render_element_ascii(child, out, indent_level + 1, max_w);
        }
    }

    /// Transitions the mockup card to `Approved` state.
    pub fn approve_mockup(card: &mut VisualMockupCard) -> Result<(), MockupError> {
        if card.approval_status == MockupApprovalState::Rejected {
            return Err(MockupError::InvalidApprovalTransition {
                from: card.approval_status.to_string(),
                to: "Approved".to_string(),
                reason: "Cannot approve a rejected mockup directly without revision".to_string(),
            });
        }

        card.approval_status = MockupApprovalState::Approved;
        Ok(())
    }

    /// Transitions the mockup card to `RevisionRequested` state with feedback notes.
    pub fn request_revision(card: &mut VisualMockupCard, notes: &str) -> Result<(), MockupError> {
        card.approval_status = MockupApprovalState::RevisionRequested;
        card.revision_notes = Some(notes.to_string());
        Ok(())
    }

    /// Rejects the mockup card.
    pub fn reject_mockup(card: &mut VisualMockupCard, reason: &str) -> Result<(), MockupError> {
        card.approval_status = MockupApprovalState::Rejected;
        card.revision_notes = Some(reason.to_string());
        Ok(())
    }
}
