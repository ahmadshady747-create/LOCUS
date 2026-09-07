//! Data types and models for the Pre-Gen Visual UI Mockup Engine.
//!
//! Provides deterministic mockup identification, layout flex hierarchies,
//! multi-platform design tokens, and interactive approval state tracking.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers and Enums
// ---------------------------------------------------------------------------

/// Deterministic 64-bit FNV-1a identifier for visual mockups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MockupId(pub u64);

impl MockupId {
    /// Generates a deterministic `MockupId` from title and framework identifier.
    pub fn new(title: &str, framework: &str) -> Self {
        let combined = format!("{}:{}", title, framework);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for MockupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mockup_{:016x}", self.0)
    }
}

/// Supported target framework environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetFramework {
    Flutter,
    ReactTailwind,
    Vue,
    Svelte,
}

impl fmt::Display for TargetFramework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Flutter => "Flutter (Material 3 / Cupertino)",
            Self::ReactTailwind => "React + Tailwind CSS",
            Self::Vue => "Vue 3 + Tailwind",
            Self::Svelte => "SvelteKit + Tailwind",
        };
        write!(f, "{}", s)
    }
}

/// Visual theme presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeMode {
    DarkCyberpunk,
    LightMinimal,
    AdaptiveSystem,
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::DarkCyberpunk => "Dark Cyberpunk",
            Self::LightMinimal => "Light Minimal",
            Self::AdaptiveSystem => "Adaptive System",
        };
        write!(f, "{}", s)
    }
}

/// Semantic and structural role of a visual component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualRole {
    AppHeader,
    NavigationBar,
    HeroCard,
    FormInput,
    ActionButton,
    DataList,
    ChartWidget,
    ModalSheet,
    StatusBadge,
}

impl fmt::Display for VisualRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AppHeader => "AppHeader",
            Self::NavigationBar => "NavigationBar",
            Self::HeroCard => "HeroCard",
            Self::FormInput => "FormInput",
            Self::ActionButton => "ActionButton",
            Self::DataList => "DataList",
            Self::ChartWidget => "ChartWidget",
            Self::ModalSheet => "ModalSheet",
            Self::StatusBadge => "StatusBadge",
        };
        write!(f, "{}", s)
    }
}

/// Flex layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutDirection {
    Row,
    Column,
}

impl fmt::Display for LayoutDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row => write!(f, "Row"),
            Self::Column => write!(f, "Column"),
        }
    }
}

// ---------------------------------------------------------------------------
// Layout and Design Tokens
// ---------------------------------------------------------------------------

/// Responsive flex layout properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutFlex {
    pub direction: LayoutDirection,
    /// (top, right, bottom, left) in logical pixels / rem units
    pub padding: (f32, f32, f32, f32),
    /// (top, right, bottom, left) in logical pixels / rem units
    pub margin: (f32, f32, f32, f32),
    pub gap: f32,
    pub expanded_factor: Option<u8>,
}

impl Default for LayoutFlex {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Column,
            padding: (0.0, 0.0, 0.0, 0.0),
            margin: (0.0, 0.0, 0.0, 0.0),
            gap: 8.0,
            expanded_factor: None,
        }
    }
}

/// Cross-platform visual styling tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignTokens {
    pub surface_color: String,
    pub text_color: String,
    pub corner_radius: f32,
    pub elevation: f32,
    pub border_style: Option<String>,
}

impl Default for DesignTokens {
    fn default() -> Self {
        Self {
            surface_color: "#1e1e2e".to_string(),
            text_color: "#cdd6f4".to_string(),
            corner_radius: 8.0,
            elevation: 1.0,
            border_style: Some("1px solid #313244".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Visual Element Tree & Mockup Card
// ---------------------------------------------------------------------------

/// A node in the visual layout tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualElement {
    pub id: String,
    pub role: VisualRole,
    pub label: Option<String>,
    pub layout: LayoutFlex,
    pub tokens: DesignTokens,
    pub children: Vec<VisualElement>,
}

impl VisualElement {
    /// Creates a leaf visual element without children.
    pub fn leaf(id: impl Into<String>, role: VisualRole, label: Option<String>, tokens: DesignTokens) -> Self {
        Self {
            id: id.into(),
            role,
            label,
            layout: LayoutFlex::default(),
            tokens,
            children: Vec::new(),
        }
    }

    /// Creates a container element with children.
    pub fn container(
        id: impl Into<String>,
        role: VisualRole,
        label: Option<String>,
        layout: LayoutFlex,
        tokens: DesignTokens,
        children: Vec<VisualElement>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            label,
            layout,
            tokens,
            children,
        }
    }

    /// Recursively counts all visual nodes in the tree.
    pub fn total_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.total_nodes()).sum::<usize>()
    }
}

/// Interactive user approval states for visual mockups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MockupApprovalState {
    PendingApproval,
    Approved,
    RevisionRequested,
    Rejected,
}

impl fmt::Display for MockupApprovalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::PendingApproval => "Pending Approval (Awaiting User Review)",
            Self::Approved => "Approved (Ready for Code Synthesis)",
            Self::RevisionRequested => "Revision Requested",
            Self::Rejected => "Rejected",
        };
        write!(f, "{}", s)
    }
}

/// Complete Visual Mockup Card payload ready for dual export and interactive preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualMockupCard {
    pub id: MockupId,
    pub title: String,
    pub framework: TargetFramework,
    pub theme: ThemeMode,
    pub root_element: VisualElement,
    pub approval_status: MockupApprovalState,
    pub created_at_ms: u64,
    pub revision_notes: Option<String>,
}

impl VisualMockupCard {
    pub fn is_approved(&self) -> bool {
        self.approval_status == MockupApprovalState::Approved
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MockupError {
    #[error("Invalid visual layout: {0}")]
    InvalidLayout(String),

    #[error("Missing root visual element in mockup")]
    MissingRootElement,

    #[error("Invalid approval state transition from {from} to {to}: {reason}")]
    InvalidApprovalTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Internal mockup engine error: {0}")]
    InternalError(String),
}
