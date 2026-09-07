//! Visual Layout Synthesizer for Pre-Gen UI Mockups.
//!
//! Synthesizes structured, responsive visual element hierarchies for Flutter and React/Web
//! from architectural contracts and component roles with theme tokens and spacing validation.

#![forbid(unsafe_code)]

use crate::ui_guard::mockup::types::{
    DesignTokens, LayoutDirection, LayoutFlex, MockupApprovalState, MockupError, MockupId,
    TargetFramework, ThemeMode, VisualElement, VisualMockupCard, VisualRole,
};

pub struct VisualLayoutSynthesizer;

impl VisualLayoutSynthesizer {
    /// Generates design tokens tailored to the selected theme preset.
    pub fn theme_tokens(theme: ThemeMode, role: VisualRole) -> DesignTokens {
        match theme {
            ThemeMode::DarkCyberpunk => match role {
                VisualRole::ActionButton => DesignTokens {
                    surface_color: "#ff007f".to_string(),
                    text_color: "#ffffff".to_string(),
                    corner_radius: 6.0,
                    elevation: 4.0,
                    border_style: Some("1px solid #00ffcc".to_string()),
                },
                VisualRole::StatusBadge => DesignTokens {
                    surface_color: "#00ffcc22".to_string(),
                    text_color: "#00ffcc".to_string(),
                    corner_radius: 4.0,
                    elevation: 1.0,
                    border_style: Some("1px solid #00ffcc".to_string()),
                },
                _ => DesignTokens {
                    surface_color: "#0f111a".to_string(),
                    text_color: "#00ffcc".to_string(),
                    corner_radius: 8.0,
                    elevation: 2.0,
                    border_style: Some("1px solid #1f2335".to_string()),
                },
            },
            ThemeMode::LightMinimal => match role {
                VisualRole::ActionButton => DesignTokens {
                    surface_color: "#2563eb".to_string(),
                    text_color: "#ffffff".to_string(),
                    corner_radius: 8.0,
                    elevation: 2.0,
                    border_style: None,
                },
                VisualRole::StatusBadge => DesignTokens {
                    surface_color: "#dbeafe".to_string(),
                    text_color: "#1e40af".to_string(),
                    corner_radius: 6.0,
                    elevation: 0.0,
                    border_style: Some("1px solid #bfdbfe".to_string()),
                },
                _ => DesignTokens {
                    surface_color: "#ffffff".to_string(),
                    text_color: "#111827".to_string(),
                    corner_radius: 12.0,
                    elevation: 1.0,
                    border_style: Some("1px solid #e5e7eb".to_string()),
                },
            },
            ThemeMode::AdaptiveSystem => match role {
                VisualRole::ActionButton => DesignTokens {
                    surface_color: "#6366f1".to_string(),
                    text_color: "#ffffff".to_string(),
                    corner_radius: 8.0,
                    elevation: 2.0,
                    border_style: None,
                },
                VisualRole::StatusBadge => DesignTokens {
                    surface_color: "#312e81".to_string(),
                    text_color: "#a5b4fc".to_string(),
                    corner_radius: 6.0,
                    elevation: 1.0,
                    border_style: Some("1px solid #4338ca".to_string()),
                },
                _ => DesignTokens {
                    surface_color: "#18181b".to_string(),
                    text_color: "#fafafa".to_string(),
                    corner_radius: 8.0,
                    elevation: 2.0,
                    border_style: Some("1px solid #27272a".to_string()),
                },
            },
        }
    }

    /// Synthesizes a standardized, production-grade Login Screen mockup.
    pub fn synthesize_login_screen(framework: TargetFramework, theme: ThemeMode) -> VisualMockupCard {
        let title = "Login Screen";
        let card_id = MockupId::new(title, &framework.to_string());

        let header = VisualElement::leaf(
            "login_header",
            VisualRole::AppHeader,
            Some("Sign In to LOCUS Swarm OS".to_string()),
            Self::theme_tokens(theme, VisualRole::AppHeader),
        );

        let email_input = VisualElement::leaf(
            "input_email",
            VisualRole::FormInput,
            Some("Email Address: user@locus.dev".to_string()),
            Self::theme_tokens(theme, VisualRole::FormInput),
        );

        let password_input = VisualElement::leaf(
            "input_password",
            VisualRole::FormInput,
            Some("Password: ••••••••••••".to_string()),
            Self::theme_tokens(theme, VisualRole::FormInput),
        );

        let submit_button = VisualElement::leaf(
            "btn_submit",
            VisualRole::ActionButton,
            Some("Authenticate & Enter".to_string()),
            Self::theme_tokens(theme, VisualRole::ActionButton),
        );

        let badge = VisualElement::leaf(
            "badge_tls",
            VisualRole::StatusBadge,
            Some("Verified Zero-Knowledge Cryptographic Lease".to_string()),
            Self::theme_tokens(theme, VisualRole::StatusBadge),
        );

        let form_container = VisualElement::container(
            "login_card_container",
            VisualRole::HeroCard,
            Some("Authentication Form".to_string()),
            LayoutFlex {
                direction: LayoutDirection::Column,
                padding: (24.0, 24.0, 24.0, 24.0),
                margin: (16.0, 16.0, 16.0, 16.0),
                gap: 16.0,
                expanded_factor: None,
            },
            Self::theme_tokens(theme, VisualRole::HeroCard),
            vec![header, email_input, password_input, submit_button, badge],
        );

        VisualMockupCard {
            id: card_id,
            title: title.to_string(),
            framework,
            theme,
            root_element: form_container,
            approval_status: MockupApprovalState::PendingApproval,
            created_at_ms: 1725000000000,
            revision_notes: None,
        }
    }

    /// Synthesizes a high-density analytics Dashboard Screen mockup.
    pub fn synthesize_dashboard_screen(framework: TargetFramework, theme: ThemeMode) -> VisualMockupCard {
        let title = "Swarm OS Dashboard";
        let card_id = MockupId::new(title, &framework.to_string());

        let header = VisualElement::leaf(
            "dash_header",
            VisualRole::AppHeader,
            Some("LOCUS Sovereign Swarm Monitor".to_string()),
            Self::theme_tokens(theme, VisualRole::AppHeader),
        );

        let nav_bar = VisualElement::container(
            "nav_bar",
            VisualRole::NavigationBar,
            Some("System Navigation".to_string()),
            LayoutFlex {
                direction: LayoutDirection::Row,
                padding: (8.0, 16.0, 8.0, 16.0),
                margin: (0.0, 0.0, 8.0, 0.0),
                gap: 12.0,
                expanded_factor: None,
            },
            Self::theme_tokens(theme, VisualRole::NavigationBar),
            vec![
                VisualElement::leaf("nav_item_overview", VisualRole::ActionButton, Some("Overview".to_string()), Self::theme_tokens(theme, VisualRole::ActionButton)),
                VisualElement::leaf("nav_item_nodes", VisualRole::ActionButton, Some("Swarm Nodes".to_string()), Self::theme_tokens(theme, VisualRole::ActionButton)),
                VisualElement::leaf("nav_item_security", VisualRole::ActionButton, Some("AST Guards".to_string()), Self::theme_tokens(theme, VisualRole::ActionButton)),
            ],
        );

        let metric_card_1 = VisualElement::leaf(
            "metric_active_nodes",
            VisualRole::HeroCard,
            Some("Active Nodes: 128 (100% Health)".to_string()),
            Self::theme_tokens(theme, VisualRole::HeroCard),
        );

        let metric_card_2 = VisualElement::leaf(
            "metric_ast_latency",
            VisualRole::HeroCard,
            Some("Mean CST Parse: 0.024ms".to_string()),
            Self::theme_tokens(theme, VisualRole::HeroCard),
        );

        let metrics_row = VisualElement::container(
            "metrics_row",
            VisualRole::HeroCard,
            Some("Key Metrics".to_string()),
            LayoutFlex {
                direction: LayoutDirection::Row,
                padding: (12.0, 12.0, 12.0, 12.0),
                margin: (0.0, 0.0, 12.0, 0.0),
                gap: 12.0,
                expanded_factor: None,
            },
            Self::theme_tokens(theme, VisualRole::HeroCard),
            vec![metric_card_1, metric_card_2],
        );

        let chart = VisualElement::leaf(
            "chart_throughput",
            VisualRole::ChartWidget,
            Some("AST Invariant Verification Velocity (req/s)".to_string()),
            Self::theme_tokens(theme, VisualRole::ChartWidget),
        );

        let tx_list = VisualElement::leaf(
            "list_transactions",
            VisualRole::DataList,
            Some("Recent ACID Workspace Transactions (12 commits)".to_string()),
            Self::theme_tokens(theme, VisualRole::DataList),
        );

        let root = VisualElement::container(
            "dash_root",
            VisualRole::HeroCard,
            Some("Dashboard Layout".to_string()),
            LayoutFlex {
                direction: LayoutDirection::Column,
                padding: (20.0, 20.0, 20.0, 20.0),
                margin: (0.0, 0.0, 0.0, 0.0),
                gap: 16.0,
                expanded_factor: None,
            },
            Self::theme_tokens(theme, VisualRole::HeroCard),
            vec![header, nav_bar, metrics_row, chart, tx_list],
        );

        VisualMockupCard {
            id: card_id,
            title: title.to_string(),
            framework,
            theme,
            root_element: root,
            approval_status: MockupApprovalState::PendingApproval,
            created_at_ms: 1725000000000,
            revision_notes: None,
        }
    }

    /// Generic layout synthesis from a custom title and component roles.
    pub fn synthesize_screen(
        title: &str,
        framework: TargetFramework,
        theme: ThemeMode,
        roles: &[VisualRole],
    ) -> Result<VisualMockupCard, MockupError> {
        if roles.is_empty() {
            return Err(MockupError::InvalidLayout(
                "Cannot synthesize screen with 0 components".to_string(),
            ));
        }

        let mut children = Vec::with_capacity(roles.len());
        for (i, &role) in roles.iter().enumerate() {
            let id = format!("elem_{}_{:?}", i, role).to_lowercase();
            let label = Some(format!("{} Component #{}", role, i + 1));
            children.push(VisualElement::leaf(
                id,
                role,
                label,
                Self::theme_tokens(theme, role),
            ));
        }

        let root = VisualElement::container(
            "screen_root",
            VisualRole::HeroCard,
            Some(title.to_string()),
            LayoutFlex {
                direction: LayoutDirection::Column,
                padding: (16.0, 16.0, 16.0, 16.0),
                margin: (0.0, 0.0, 0.0, 0.0),
                gap: 12.0,
                expanded_factor: None,
            },
            Self::theme_tokens(theme, VisualRole::HeroCard),
            children,
        );

        let card_id = MockupId::new(title, &framework.to_string());

        Ok(VisualMockupCard {
            id: card_id,
            title: title.to_string(),
            framework,
            theme,
            root_element: root,
            approval_status: MockupApprovalState::PendingApproval,
            created_at_ms: 1725000000000,
            revision_notes: None,
        })
    }

    /// Validates visual tree layout integrity (no negative or NaN margins/paddings).
    pub fn validate_layout_integrity(card: &VisualMockupCard) -> Result<(), MockupError> {
        fn check_element(elem: &VisualElement) -> Result<(), MockupError> {
            let (pt, pr, pb, pl) = elem.layout.padding;
            if pt < 0.0 || pr < 0.0 || pb < 0.0 || pl < 0.0 || pt.is_nan() || pr.is_nan() || pb.is_nan() || pl.is_nan() {
                return Err(MockupError::InvalidLayout(format!(
                    "Element '{}' contains negative or NaN padding: {:?}",
                    elem.id, elem.layout.padding
                )));
            }

            let (mt, mr, mb, ml) = elem.layout.margin;
            if mt < 0.0 || mr < 0.0 || mb < 0.0 || ml < 0.0 || mt.is_nan() || mr.is_nan() || mb.is_nan() || ml.is_nan() {
                return Err(MockupError::InvalidLayout(format!(
                    "Element '{}' contains negative or NaN margin: {:?}",
                    elem.id, elem.layout.margin
                )));
            }

            if elem.layout.gap < 0.0 || elem.layout.gap.is_nan() {
                return Err(MockupError::InvalidLayout(format!(
                    "Element '{}' contains negative gap: {}",
                    elem.id, elem.layout.gap
                )));
            }

            for child in &elem.children {
                check_element(child)?;
            }

            Ok(())
        }

        check_element(&card.root_element)
    }
}
