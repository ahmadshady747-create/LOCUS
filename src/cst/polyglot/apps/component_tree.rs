//! Component Tree Extractor for Frontend UI Frameworks (React, TSX, Flutter/Dart).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use crate::cst::green::TextRange;
use crate::types::Language;

/// Layout role of a component in UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutRole {
    Container,
    Row,
    Column,
    Grid,
    Card,
    Text,
    Button,
    Input,
    Custom,
}

/// Category of UI component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    HostElement,
    CustomComponent,
    Widget,
    Fragment,
}

/// An extracted property or attribute on a UI component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentProp {
    pub name: String,
    pub value: String,
    pub is_dynamic: bool,
}

/// A node in the extracted hierarchical component tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentNode {
    pub name: String,
    pub component_type: ComponentType,
    pub layout_role: LayoutRole,
    pub props: Vec<ComponentProp>,
    pub children: Vec<ComponentNode>,
    pub range: TextRange,
}

impl ComponentNode {
    pub fn new(name: impl Into<String>, component_type: ComponentType, layout_role: LayoutRole, range: TextRange) -> Self {
        Self {
            name: name.into(),
            component_type,
            layout_role,
            props: Vec::new(),
            children: Vec::new(),
            range,
        }
    }
}

/// Extractor for hierarchical UI component trees.
#[derive(Debug, Default, Clone, Copy)]
pub struct ComponentTreeExtractor;

impl ComponentTreeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extracts all component trees from source code based on language.
    pub fn extract(&self, source: &str, lang: Language) -> Vec<ComponentNode> {
        let mut roots = Vec::new();
        match lang {
            Language::Tsx | Language::Jsx | Language::TypeScript | Language::JavaScript => {
                // Parse JSX tags
                let mut stack: Vec<ComponentNode> = Vec::new();
                for line in source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('<') && !trimmed.starts_with("</") && !trimmed.starts_with("<!--") {
                        let tag_content = trimmed.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
                        let mut parts = tag_content.split_whitespace();
                        if let Some(tag_name) = parts.next() {
                            let role = match tag_name {
                                "div" => LayoutRole::Container,
                                "Header" | "HeaderBar" => LayoutRole::Custom,
                                "Card" => LayoutRole::Card,
                                "button" => LayoutRole::Button,
                                "p" | "span" | "h1" | "h2" => LayoutRole::Text,
                                _ => LayoutRole::Custom,
                            };
                            let ctype = if tag_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                                ComponentType::CustomComponent
                            } else {
                                ComponentType::HostElement
                            };

                            let mut node = ComponentNode::new(tag_name, ctype, role, TextRange::new(0, 0));
                            for prop_part in parts {
                                if let Some((k, v)) = prop_part.split_once('=') {
                                    node.props.push(ComponentProp {
                                        name: k.to_string(),
                                        value: v.trim_matches('"').trim_matches('\'').to_string(),
                                        is_dynamic: v.starts_with('{'),
                                    });
                                }
                            }

                            if trimmed.ends_with("/>") {
                                if let Some(parent) = stack.last_mut() {
                                    parent.children.push(node);
                                } else {
                                    roots.push(node);
                                }
                            } else {
                                stack.push(node);
                            }
                        }
                    } else if trimmed.starts_with("</") {
                        if let Some(closed_node) = stack.pop() {
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(closed_node);
                            } else {
                                roots.push(closed_node);
                            }
                        }
                    }
                }
                while let Some(closed_node) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(closed_node);
                    } else {
                        roots.push(closed_node);
                    }
                }
            }
            Language::Dart => {
                // Parse Flutter Widget trees
                let widgets = ["Scaffold", "AppBar", "Center", "Column", "Row", "Text", "ElevatedButton"];
                let mut stack: Vec<ComponentNode> = Vec::new();
                for line in source.lines() {
                    let trimmed = line.trim();
                    for &w in &widgets {
                        if trimmed.contains(&format!("{}(", w)) || trimmed.contains(&format!("{} (", w)) {
                            let role = match w {
                                "Column" => LayoutRole::Column,
                                "Row" => LayoutRole::Row,
                                "Center" | "Scaffold" => LayoutRole::Container,
                                "Text" => LayoutRole::Text,
                                "ElevatedButton" => LayoutRole::Button,
                                _ => LayoutRole::Custom,
                            };
                            let node = ComponentNode::new(w, ComponentType::Widget, role, TextRange::new(0, 0));
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(node);
                            } else {
                                stack.push(node);
                            }
                            break;
                        }
                    }
                }
                roots = stack;
            }
            _ => {}
        }

        roots
    }

    /// Computes total number of nodes in forest.
    pub fn total_nodes(&self, roots: &[ComponentNode]) -> usize {
        let mut count = 0;
        fn visit(node: &ComponentNode, c: &mut usize) {
            *c += 1;
            for child in &node.children {
                visit(child, c);
            }
        }
        for r in roots {
            visit(r, &mut count);
        }
        count
    }

    /// Finds all components matching given name.
    pub fn find_by_name<'a>(&self, roots: &'a [ComponentNode], name: &str) -> Vec<&'a ComponentNode> {
        let mut matched = Vec::new();
        fn visit<'a>(node: &'a ComponentNode, name: &str, res: &mut Vec<&'a ComponentNode>) {
            if node.name == name {
                res.push(node);
            }
            for child in &node.children {
                visit(child, name, res);
            }
        }
        for r in roots {
            visit(r, name, &mut matched);
        }
        matched
    }
}
