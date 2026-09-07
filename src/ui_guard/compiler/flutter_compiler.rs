//! Production-Ready Flutter / Dart UI Compiler.
//!
//! Transforms approved `VisualMockupCard` trees into idiomatic, performance-optimized
//! Material 3 Flutter `StatelessWidget` code and synthesizes a lossless `GreenTreeRoot`.

#![forbid(unsafe_code)]

use crate::cst::polyglot::drivers::DartCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::ui_guard::compiler::types::{CompiledUiArtifact, UiCompilerConfig, UiCompilerError};
use crate::ui_guard::invariants::flutter_solver::FlutterLayoutSolver;
use crate::ui_guard::mockup::types::{
    LayoutDirection, LayoutFlex, VisualElement, VisualMockupCard, VisualRole,
};
use std::time::Instant;

/// Compiler transforming visual mockup trees into production Flutter Dart code.
#[derive(Debug, Default, Clone, Copy)]
pub struct FlutterUiCompiler;

impl FlutterUiCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compiles an approved visual mockup card into a verified Flutter artifact.
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

        let file_name = format!("{}.dart", Self::to_snake_case(&component_name));

        // 2. Generate Dart source code
        let formatted_source = self.generate_flutter_source(mockup, &component_name, config);

        // 3. Verify invariants using FlutterLayoutSolver
        let solver = FlutterLayoutSolver::new();
        let invariant_report = solver.verify(&formatted_source);

        if !invariant_report.passed {
            return Err(UiCompilerError::LayoutViolationDetected(
                invariant_report.violations.clone(),
            ));
        }

        // 4. Synthesize GreenTreeRoot via DartCstDriver
        let driver = DartCstDriver;
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

    /// Generates complete formatted Flutter source code.
    fn generate_flutter_source(
        &self,
        mockup: &VisualMockupCard,
        component_name: &str,
        config: &UiCompilerConfig,
    ) -> String {
        let mut out = String::with_capacity(2048);

        // Imports
        out.push_str("import 'package:flutter/material.dart';\n\n");

        // Documentation comment
        if config.generate_comments {
            out.push_str(&format!(
                "/// Production-ready Flutter widget for [{}].\n",
                mockup.title
            ));
            out.push_str("/// Synthesized deterministically by LOCUS UI Compiler.\n");
        }

        // Widget Class
        out.push_str(&format!(
            "class {} extends StatelessWidget {{\n",
            component_name
        ));

        // Constructor
        if config.use_const_constructors {
            out.push_str(&format!(
                "  const {}({{super.key}});\n\n",
                component_name
            ));
        } else {
            out.push_str(&format!(
                "  {}({{Key? key}}) : super(key: key);\n\n",
                component_name
            ));
        }

        // Build method
        out.push_str("  @override\n");
        out.push_str("  Widget build(BuildContext context) {\n");

        let widget_body = self.compile_element(&mockup.root_element, 2, config);
        out.push_str(&widget_body);
        out.push_str(";\n");
        out.push_str("  }\n");
        out.push_str("}\n");

        out
    }

    /// Recursively compiles a `VisualElement` into a Flutter widget tree expression.
    fn compile_element(
        &self,
        element: &VisualElement,
        indent_level: usize,
        config: &UiCompilerConfig,
    ) -> String {
        let pad = "  ".repeat(indent_level);

        match element.role {
            VisualRole::HeroCard => {
                let inner_pad = "  ".repeat(indent_level + 1);
                let child_expr = if !element.children.is_empty() {
                    self.compile_container_children(&element.children, &element.layout, indent_level + 2, config)
                } else {
                    format!("{}const SizedBox.shrink()", "  ".repeat(indent_level + 2))
                };

                let color_val = Self::hex_to_flutter_color(&element.tokens.surface_color);
                let radius = element.tokens.corner_radius;
                let elevation = element.tokens.elevation;

                format!(
                    "{}Card(\n{}elevation: {:.1},\n{}shape: RoundedRectangleBorder(\n{}  borderRadius: BorderRadius.circular({:.1}),\n{}),\n{}color: {},\n{}child: Padding(\n{}  padding: const EdgeInsets.fromLTRB({:.1}, {:.1}, {:.1}, {:.1}),\n{}  child: \n{},\n{}),\n{})",
                    pad,
                    inner_pad,
                    elevation,
                    inner_pad,
                    inner_pad,
                    radius,
                    inner_pad,
                    inner_pad,
                    color_val,
                    inner_pad,
                    inner_pad,
                    element.layout.padding.3,
                    element.layout.padding.0,
                    element.layout.padding.1,
                    element.layout.padding.2,
                    inner_pad,
                    child_expr,
                    inner_pad,
                    pad
                )
            }
            VisualRole::ActionButton => {
                let label_text = element.label.as_deref().unwrap_or("Submit");
                let color_val = Self::hex_to_flutter_color(&element.tokens.surface_color);
                let text_color_val = Self::hex_to_flutter_color(&element.tokens.text_color);

                format!(
                    "{}FilledButton(\n{}  onPressed: () {{}},\n{}  style: FilledButton.styleFrom(\n{}    backgroundColor: {},\n{}    foregroundColor: {},\n{}    shape: RoundedRectangleBorder(\n{}      borderRadius: BorderRadius.circular({:.1}),\n{}    ),\n{}  ),\n{}  child: const Text('{}'),\n{})",
                    pad,
                    pad,
                    pad,
                    pad,
                    color_val,
                    pad,
                    text_color_val,
                    pad,
                    pad,
                    element.tokens.corner_radius,
                    pad,
                    pad,
                    pad,
                    label_text,
                    pad
                )
            }
            VisualRole::FormInput => {
                let label_text = element.label.as_deref().unwrap_or("Input Field");
                format!(
                    "{}TextField(\n{}  decoration: InputDecoration(\n{}    labelText: '{}',\n{}    border: OutlineInputBorder(\n{}      borderRadius: BorderRadius.circular({:.1}),\n{}    ),\n{}  ),\n{})",
                    pad,
                    pad,
                    pad,
                    label_text,
                    pad,
                    pad,
                    element.tokens.corner_radius,
                    pad,
                    pad,
                    pad
                )
            }
            VisualRole::StatusBadge => {
                let text_val = element.label.as_deref().unwrap_or("Active");
                let color_val = Self::hex_to_flutter_color(&element.tokens.surface_color);
                let text_color = Self::hex_to_flutter_color(&element.tokens.text_color);

                format!(
                    "{}Container(\n{}  padding: const EdgeInsets.symmetric(horizontal: 10.0, vertical: 4.0),\n{}  decoration: BoxDecoration(\n{}    color: {},\n{}    borderRadius: BorderRadius.circular({:.1}),\n{}  ),\n{}  child: Text(\n{}    '{}',\n{}    style: TextStyle(color: {}, fontWeight: FontWeight.bold),\n{}  ),\n{})",
                    pad,
                    pad,
                    pad,
                    pad,
                    color_val,
                    pad,
                    element.tokens.corner_radius,
                    pad,
                    pad,
                    pad,
                    text_val,
                    pad,
                    text_color,
                    pad,
                    pad
                )
            }
            VisualRole::DataList => {
                // If DataList is in flex, use shrinkWrap and NeverScrollableScrollPhysics to prevent RenderFlex overflow
                if !element.children.is_empty() {
                    let mut list_out = format!(
                        "{}ListView(\n{}  shrinkWrap: true,\n{}  physics: const NeverScrollableScrollPhysics(),\n{}  children: [\n",
                        pad, pad, pad, pad
                    );
                    for child in &element.children {
                        let child_str = self.compile_element(child, indent_level + 2, config);
                        list_out.push_str(&format!("{},\n", child_str));
                    }
                    list_out.push_str(&format!("{}  ],\n{})", pad, pad));
                    list_out
                } else {
                    format!("{}const SizedBox.shrink()", pad)
                }
            }
            _ => {
                // Generic container or layout block
                if !element.children.is_empty() {
                    self.compile_container_children(&element.children, &element.layout, indent_level, config)
                } else if let Some(ref lbl) = element.label {
                    let text_color = Self::hex_to_flutter_color(&element.tokens.text_color);
                    format!(
                        "{}Text(\n{}  '{}',\n{}  style: TextStyle(color: {}),\n{})",
                        pad, pad, lbl, pad, text_color, pad
                    )
                } else {
                    format!("{}const SizedBox.shrink()", pad)
                }
            }
        }
    }

    /// Compiles container children based on LayoutFlex (Column or Row).
    fn compile_container_children(
        &self,
        children: &[VisualElement],
        layout: &LayoutFlex,
        indent_level: usize,
        config: &UiCompilerConfig,
    ) -> String {
        let pad = "  ".repeat(indent_level);
        let flex_type = match layout.direction {
            LayoutDirection::Column => "Column",
            LayoutDirection::Row => "Row",
        };

        let mut out = format!(
            "{}{}(\n{}  mainAxisSize: MainAxisSize.min,\n{}  crossAxisAlignment: CrossAxisAlignment.start,\n{}  children: [\n",
            pad, flex_type, pad, pad, pad
        );

        for (idx, child) in children.iter().enumerate() {
            let child_code = self.compile_element(child, indent_level + 2, config);
            out.push_str(&child_code);
            out.push_str(",\n");

            // Add gap spacing between elements
            if layout.gap > 0.0 && idx < children.len() - 1 {
                let spacer = match layout.direction {
                    LayoutDirection::Column => {
                        format!("{}  SizedBox(height: {:.1}),\n", pad, layout.gap)
                    }
                    LayoutDirection::Row => {
                        format!("{}  SizedBox(width: {:.1}),\n", pad, layout.gap)
                    }
                };
                out.push_str(&spacer);
            }
        }

        out.push_str(&format!("{}  ],\n{})", pad, pad));
        out
    }

    /// Converts hex string `#RRGGBB` or `#RRGGBBAA` into `Color(0xAARRGGBB)`.
    fn hex_to_flutter_color(hex: &str) -> String {
        let clean = hex.trim_start_matches('#');
        match clean.len() {
            6 => format!("Color(0xFF{})", clean.to_uppercase()),
            8 => {
                // RRGGBBAA -> AARRGGBB
                let rgb = &clean[0..6];
                let alpha = &clean[6..8];
                format!("Color(0x{}{})", alpha.to_uppercase(), rgb.to_uppercase())
            }
            _ => "Color(0xFF2563EB)".to_string(),
        }
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
            "GeneratedWidget".to_string()
        } else {
            res
        }
    }

    /// Converts a PascalCase or Title string to snake_case.
    fn to_snake_case(s: &str) -> String {
        let mut res = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    res.push('_');
                }
                res.push(ch.to_ascii_lowercase());
            } else if ch.is_alphanumeric() {
                res.push(ch);
            } else {
                res.push('_');
            }
        }
        res
    }
}
