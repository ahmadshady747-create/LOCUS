//! Production-Ready React / TSX + Tailwind CSS UI Compiler.
//!
//! Transforms approved `VisualMockupCard` trees into idiomatic, fully-typed
//! modern React functional components with Tailwind CSS utility classes.

#![forbid(unsafe_code)]

use crate::cst::polyglot::drivers::TypeScriptCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::ui_guard::compiler::types::{
    CompiledUiArtifact, ComponentExportStyle, UiCompilerConfig, UiCompilerError,
};
use crate::ui_guard::invariants::tsx_solver::TsxLayoutSolver;
use crate::ui_guard::mockup::types::{
    DesignTokens, LayoutDirection, LayoutFlex, VisualElement, VisualMockupCard, VisualRole,
};
use std::time::Instant;

/// Compiler transforming visual mockup trees into production React TSX code.
#[derive(Debug, Default, Clone, Copy)]
pub struct TsxUiCompiler;

impl TsxUiCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compiles an approved visual mockup card into a verified React TSX artifact.
    pub fn compile(
        &self,
        mockup: &VisualMockupCard,
        config: &UiCompilerConfig,
    ) -> Result<CompiledUiArtifact, UiCompilerError> {
        let start = Instant::now();

        // 1. Determine PascalCase component name
        let component_name = config
            .component_name
            .clone()
            .unwrap_or_else(|| Self::to_pascal_case(&mockup.title));

        let file_name = format!("{}.tsx", component_name);

        // 2. Generate TSX source code
        let formatted_source = self.generate_tsx_source(mockup, &component_name, config);

        // 3. Verify invariants using TsxLayoutSolver
        let solver = TsxLayoutSolver::new();
        let invariant_report = solver.verify(&formatted_source);

        if !invariant_report.passed {
            return Err(UiCompilerError::LayoutViolationDetected(
                invariant_report.violations.clone(),
            ));
        }

        // 4. Synthesize GreenTreeRoot via TypeScriptCstDriver
        let driver = TypeScriptCstDriver::new(crate::cst::polyglot::types::LanguageId::Tsx);
        let cst_tree = driver
            .parse_lossless(&formatted_source)
            .map_err(|e| UiCompilerError::CstSynthesisFailed(e.to_string()))?;

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        Ok(CompiledUiArtifact {
            file_name,
            cst_tree,
            formatted_source,
            invariants_verified: true,
            latency_ms: elapsed,
            invariant_report,
        })
    }

    /// Generates complete formatted React/TSX source code.
    fn generate_tsx_source(
        &self,
        mockup: &VisualMockupCard,
        component_name: &str,
        config: &UiCompilerConfig,
    ) -> String {
        let mut out = String::with_capacity(2048);

        // Imports
        out.push_str("import React from 'react';\n\n");

        // Props Interface
        out.push_str(&format!("export interface {}Props {{\n", component_name));
        out.push_str("  className?: string;\n");
        out.push_str("  children?: React.ReactNode;\n");
        out.push_str("}\n\n");

        // Documentation comment
        if config.generate_comments {
            out.push_str(&format!(
                "/**\n * Production-ready React component for [{}].\n * Synthesized deterministically by LOCUS UI Compiler with Tailwind CSS.\n */\n",
                mockup.title
            ));
        }

        // Function Declaration
        let export_prefix = match config.export_style {
            ComponentExportStyle::NamedExport => "export function",
            ComponentExportStyle::DefaultExport => "export default function",
        };

        out.push_str(&format!(
            "{} {}({{ className = '', children }}: {}Props) {{\n",
            export_prefix, component_name, component_name
        ));
        out.push_str("  return (\n");

        let body = self.compile_element(&mockup.root_element, 2);
        out.push_str(&body);
        out.push_str("\n  );\n}\n");

        out
    }

    /// Recursively compiles a `VisualElement` into TSX JSX tags with Tailwind classes.
    fn compile_element(&self, element: &VisualElement, indent_level: usize) -> String {
        let pad = "  ".repeat(indent_level);

        match element.role {
            VisualRole::HeroCard => {
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "card");
                let mut out = format!("{}<article className=\"{}\">\n", pad, tailwind_classes);
                if !element.children.is_empty() {
                    for child in &element.children {
                        out.push_str(&self.compile_element(child, indent_level + 1));
                        out.push('\n');
                    }
                } else if let Some(ref lbl) = element.label {
                    out.push_str(&format!("{}  <h3 className=\"text-lg font-bold\">{}</h3>\n", pad, lbl));
                }
                out.push_str(&format!("{}</article>", pad));
                out
            }
            VisualRole::ActionButton => {
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "button");
                let label_text = element.label.as_deref().unwrap_or("Submit");
                format!(
                    "{}<button type=\"button\" className=\"{}\">\n{}  {}\n{}</button>",
                    pad, tailwind_classes, pad, label_text, pad
                )
            }
            VisualRole::FormInput => {
                let label_text = element.label.as_deref().unwrap_or("Input Field");
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "input");
                format!(
                    "{}<div className=\"flex flex-col gap-1.5 w-full\">\n{}  <label className=\"text-xs font-semibold text-slate-300\">{}</label>\n{}  <input type=\"text\" placeholder=\"{}\" className=\"{}\" />\n{}</div>",
                    pad, pad, label_text, pad, label_text, tailwind_classes, pad
                )
            }
            VisualRole::StatusBadge => {
                let label_text = element.label.as_deref().unwrap_or("Active");
                format!(
                    "{}<span className=\"inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20\">\n{}  {}\n{}</span>",
                    pad, pad, label_text, pad
                )
            }
            VisualRole::AppHeader => {
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "header");
                let label_text = element.label.as_deref().unwrap_or("App Header");
                format!(
                    "{}<header className=\"{}\">\n{}  <h1 className=\"text-xl font-bold text-slate-100\">{}</h1>\n{}</header>",
                    pad, tailwind_classes, pad, label_text, pad
                )
            }
            VisualRole::NavigationBar => {
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "nav");
                let mut out = format!("{}<nav className=\"{}\">\n", pad, tailwind_classes);
                if !element.children.is_empty() {
                    for child in &element.children {
                        out.push_str(&self.compile_element(child, indent_level + 1));
                        out.push('\n');
                    }
                }
                out.push_str(&format!("{}</nav>", pad));
                out
            }
            VisualRole::DataList => {
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "list");
                if !element.children.is_empty() {
                    let mut out = format!("{}<ul className=\"{}\">\n", pad, tailwind_classes);
                    for (idx, child) in element.children.iter().enumerate() {
                        out.push_str(&format!("{}  <li key=\"item_{}\" className=\"list-none\">\n", pad, idx));
                        out.push_str(&self.compile_element(child, indent_level + 2));
                        out.push_str(&format!("\n{}  </li>\n", pad));
                    }
                    out.push_str(&format!("{}</ul>", pad));
                    out
                } else {
                    format!("{}<ul className=\"{}\" />", pad, tailwind_classes)
                }
            }
            _ => {
                // Generic layout block / div
                let tailwind_classes = Self::tokens_to_tailwind(&element.layout, &element.tokens, "container");
                if !element.children.is_empty() {
                    let mut out = format!("{}<div className=\"{}\">\n", pad, tailwind_classes);
                    for child in &element.children {
                        out.push_str(&self.compile_element(child, indent_level + 1));
                        out.push('\n');
                    }
                    out.push_str(&format!("{}</div>", pad));
                    out
                } else if let Some(ref lbl) = element.label {
                    format!("{}<p className=\"text-sm text-slate-300\">{}</p>", pad, lbl)
                } else {
                    format!("{}<div className=\"{}\" />", pad, tailwind_classes)
                }
            }
        }
    }

    /// Translates layout and design tokens to standard Tailwind CSS utility classes.
    fn tokens_to_tailwind(layout: &LayoutFlex, tokens: &DesignTokens, role_hint: &str) -> String {
        let mut classes = Vec::with_capacity(8);

        // 1. Flex & Direction
        match layout.direction {
            LayoutDirection::Column => classes.push("flex flex-col"),
            LayoutDirection::Row => classes.push("flex flex-row items-center"),
        }

        // 2. Gap
        if layout.gap >= 24.0 {
            classes.push("gap-6");
        } else if layout.gap >= 16.0 {
            classes.push("gap-4");
        } else if layout.gap >= 12.0 {
            classes.push("gap-3");
        } else if layout.gap >= 8.0 {
            classes.push("gap-2");
        } else if layout.gap > 0.0 {
            classes.push("gap-1");
        }

        // 3. Padding
        let (top, right, bottom, left) = layout.padding;
        if top == right && right == bottom && bottom == left {
            if top >= 24.0 {
                classes.push("p-6");
            } else if top >= 16.0 {
                classes.push("p-4");
            } else if top >= 8.0 {
                classes.push("p-2");
            }
        } else {
            if top == bottom && right == left {
                if top >= 16.0 {
                    classes.push("py-4");
                } else if top >= 8.0 {
                    classes.push("py-2");
                }
                if right >= 16.0 {
                    classes.push("px-4");
                } else if right >= 8.0 {
                    classes.push("px-2");
                }
            } else {
                classes.push("p-4");
            }
        }

        // 4. Corner Radius
        if tokens.corner_radius >= 16.0 {
            classes.push("rounded-2xl");
        } else if tokens.corner_radius >= 12.0 {
            classes.push("rounded-xl");
        } else if tokens.corner_radius >= 8.0 {
            classes.push("rounded-lg");
        } else if tokens.corner_radius >= 4.0 {
            classes.push("rounded-md");
        }

        // 5. Elevation
        if tokens.elevation >= 8.0 {
            classes.push("shadow-xl");
        } else if tokens.elevation >= 4.0 {
            classes.push("shadow-md");
        } else if tokens.elevation >= 1.0 {
            classes.push("shadow-sm");
        }

        // 6. Role-specific base styles
        match role_hint {
            "button" => {
                classes.push("px-4 py-2 text-sm font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors duration-150 focus:outline-none focus:ring-2 focus:ring-indigo-400 focus:ring-offset-2 focus:ring-offset-slate-900");
            }
            "input" => {
                classes.push("px-3 py-2 text-sm bg-slate-900/50 border border-slate-700 text-slate-100 placeholder-slate-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500");
            }
            "card" => {
                classes.push("bg-slate-900/80 border border-slate-800 backdrop-blur-sm text-slate-100");
            }
            "header" => {
                classes.push("w-full border-b border-slate-800 bg-slate-900/60 backdrop-blur-md");
            }
            "nav" => {
                classes.push("w-full border-t border-slate-800 bg-slate-900/40");
            }
            _ => {}
        }

        classes.join(" ")
    }

    /// Converts a title string to PascalCase.
    fn to_pascal_case(s: &str) -> String {
        let mut res = String::new();
        let mut capitalize_next = true;
        for ch in s.chars() {
            if ch.is_uppercase() {
                res.push(ch);
                capitalize_next = false;
            } else if ch.is_alphanumeric() {
                if capitalize_next {
                    res.push(ch.to_ascii_uppercase());
                    capitalize_next = false;
                } else {
                    res.push(ch);
                }
            } else {
                capitalize_next = true;
            }
        }
        if res.is_empty() {
            "GeneratedComponent".to_string()
        } else {
            res
        }
    }
}
