//! Specification Alignment & Architectural Tradeoff Gate
//!
//! Analyzes incoming developer goals before Task Graph DAG generation to detect
//! architectural ambiguities (state management, persistence, concurrency) and presents
//! 2-3 concise, mutually exclusive trade-off options with pros and cons to eliminate token waste.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeoffCategory {
    StateManagement,
    Persistence,
    Concurrency,
    NetworkTransport,
    ErrorStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecTradeoffOption {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecAmbiguity {
    pub id: String,
    pub category: TradeoffCategory,
    pub question: String,
    pub options: Vec<SpecTradeoffOption>,
    pub selected_option_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecAlignmentReport {
    pub goal: String,
    pub has_ambiguity: bool,
    pub ambiguities: Vec<SpecAmbiguity>,
    pub aligned_constraints: Vec<String>,
}

pub struct SpecAligner;

impl SpecAligner {
    /// Analyzes a developer goal and identifies major architectural ambiguities
    pub fn analyze_goal(goal: &str, _workspace_summary: Option<&str>) -> SpecAlignmentReport {
        let goal_lower = goal.to_lowercase();
        let mut ambiguities = Vec::new();
        let mut aligned_constraints = Vec::new();

        // 1. State Management Ambiguity Check
        if goal_lower.contains("state")
            || goal_lower.contains("store")
            || goal_lower.contains("global data")
            || goal_lower.contains("manage data")
        {
            ambiguities.push(SpecAmbiguity {
                id: "amb_state_management".to_string(),
                category: TradeoffCategory::StateManagement,
                question: "Which state management pattern should be adopted?".to_string(),
                options: vec![
                    SpecTradeoffOption {
                        id: "state_local_context".to_string(),
                        title: "Local Component State / React Context".to_string(),
                        description: "Use native React useState & useContext with zero external bundle overhead.".to_string(),
                        pros: vec!["Zero dependencies".to_string(), "Simple scoping".to_string()],
                        cons: vec!["Context re-renders if state grows large".to_string()],
                        recommended: true,
                    },
                    SpecTradeoffOption {
                        id: "state_zustand_store".to_string(),
                        title: "Zustand Micro-Store".to_string(),
                        description: "Atomic store with fine-grained selector subscriptions.".to_string(),
                        pros: vec!["Optimal re-rendering performance".to_string(), "Clean decoupled actions".to_string()],
                        cons: vec!["Adds zustand dependency".to_string()],
                        recommended: false,
                    },
                ],
                selected_option_id: Some("state_local_context".to_string()),
            });
        }

        // 2. Persistence / Storage Ambiguity Check
        if goal_lower.contains("persist")
            || goal_lower.contains("cache")
            || goal_lower.contains("save")
            || goal_lower.contains("database")
            || goal_lower.contains("store to disk")
            || goal_lower.contains("history")
        {
            ambiguities.push(SpecAmbiguity {
                id: "amb_persistence_strategy".to_string(),
                category: TradeoffCategory::Persistence,
                question: "How should data persistence and caching be handled?".to_string(),
                options: vec![
                    SpecTradeoffOption {
                        id: "persist_atomic_json".to_string(),
                        title: "Atomic Local JSON File (.locus/data.json)".to_string(),
                        description: "Human-readable JSON files with atomic rename writes.".to_string(),
                        pros: vec!["Zero DB dependencies".to_string(), "Human inspectable and easy to backup".to_string()],
                        cons: vec!["Not suitable for >100MB high-frequency writes".to_string()],
                        recommended: true,
                    },
                    SpecTradeoffOption {
                        id: "persist_in_memory_only".to_string(),
                        title: "In-Memory LRU Cache (Non-Persistent)".to_string(),
                        description: "Fast in-memory RAM cache cleared on app restart.".to_string(),
                        pros: vec!["Ultra-low latency (<1ms)".to_string(), "Zero disk I/O".to_string()],
                        cons: vec!["State lost upon restart".to_string()],
                        recommended: false,
                    },
                ],
                selected_option_id: Some("persist_atomic_json".to_string()),
            });
        }

        // 3. Concurrency / Execution Model Ambiguity Check
        if goal_lower.contains("parallel")
            || goal_lower.contains("concurrent")
            || goal_lower.contains("background")
            || goal_lower.contains("worker")
            || goal_lower.contains("batch process")
        {
            ambiguities.push(SpecAmbiguity {
                id: "amb_concurrency_model".to_string(),
                category: TradeoffCategory::Concurrency,
                question: "Which concurrency execution model fits best?".to_string(),
                options: vec![
                    SpecTradeoffOption {
                        id: "conc_tokio_async".to_string(),
                        title: "Tokio Async Tasks (Green Threads)".to_string(),
                        description: "Asynchronous task multiplexing via tokio::spawn for I/O-bound operations.".to_string(),
                        pros: vec!["Very low memory footprint".to_string(), "Excellent for I/O & network".to_string()],
                        cons: vec!["Requires Send + Sync closures".to_string()],
                        recommended: true,
                    },
                    SpecTradeoffOption {
                        id: "conc_rayon_threadpool".to_string(),
                        title: "Rayon Parallel Iterator (CPU Bound)".to_string(),
                        description: "Work-stealing OS thread pool optimized for heavy mathematical computation.".to_string(),
                        pros: vec!["Max CPU utilization across cores".to_string(), "Linear speedup for parsing".to_string()],
                        cons: vec!["Higher thread scheduling overhead".to_string()],
                        recommended: false,
                    },
                ],
                selected_option_id: Some("conc_tokio_async".to_string()),
            });
        }

        let has_ambiguity = !ambiguities.is_empty();

        // If no ambiguity, populate default solid architectural constraints
        if !has_ambiguity {
            aligned_constraints.push("Use standard workspace modular conventions with clean interfaces.".to_string());
            aligned_constraints.push("Prefer pure localized functions with zero unnecessary global mutations.".to_string());
        } else {
            for amb in &ambiguities {
                if let Some(ref opt_id) = amb.selected_option_id {
                    if let Some(opt) = amb.options.iter().find(|o| &o.id == opt_id) {
                        aligned_constraints.push(format!("Architecture choice ({}): {}", opt.title, opt.description));
                    }
                }
            }
        }

        SpecAlignmentReport {
            goal: goal.to_string(),
            has_ambiguity,
            ambiguities,
            aligned_constraints,
        }
    }

    /// Applies chosen options from user feedback and updates aligned constraints
    pub fn apply_tradeoff_choices(
        report: &mut SpecAlignmentReport,
        selections: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut new_constraints = Vec::new();

        for amb in &mut report.ambiguities {
            if let Some(chosen_id) = selections.get(&amb.id) {
                amb.selected_option_id = Some(chosen_id.clone());
            }
            if let Some(ref opt_id) = amb.selected_option_id {
                if let Some(opt) = amb.options.iter().find(|o| &o.id == opt_id) {
                    new_constraints.push(format!("Architecture constraint ({}): {}", opt.title, opt.description));
                }
            }
        }

        report.aligned_constraints = new_constraints.clone();
        new_constraints
    }

    /// Quick bypass: automatically adopts the recommended options for all detected ambiguities
    pub fn quick_decompose_defaults(report: &mut SpecAlignmentReport) -> Vec<String> {
        let mut selections = HashMap::new();
        for amb in &report.ambiguities {
            if let Some(rec) = amb.options.iter().find(|o| o.recommended) {
                selections.insert(amb.id.clone(), rec.id.clone());
            } else if let Some(first) = amb.options.first() {
                selections.insert(amb.id.clone(), first.id.clone());
            }
        }
        Self::apply_tradeoff_choices(report, &selections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_state_management_ambiguity() {
        let goal = "Implement a state store to manage active session data across tabs";
        let report = SpecAligner::analyze_goal(goal, None);

        assert!(report.has_ambiguity);
        assert!(report.ambiguities.iter().any(|a| a.category == TradeoffCategory::StateManagement));
        let state_amb = report.ambiguities.iter().find(|a| a.category == TradeoffCategory::StateManagement).unwrap();
        assert_eq!(state_amb.options.len(), 2);
        assert!(state_amb.options.iter().any(|o| o.recommended));
    }

    #[test]
    fn test_detect_persistence_ambiguity() {
        let goal = "Add caching and persist chat history to local disk";
        let report = SpecAligner::analyze_goal(goal, None);

        assert!(report.has_ambiguity);
        assert!(report.ambiguities.iter().any(|a| a.category == TradeoffCategory::Persistence));
    }

    #[test]
    fn test_clear_goal_no_ambiguity() {
        let goal = "Fix typo in button label in Settings modal";
        let report = SpecAligner::analyze_goal(goal, None);

        assert!(!report.has_ambiguity);
        assert_eq!(report.ambiguities.len(), 0);
        assert!(!report.aligned_constraints.is_empty());
    }

    #[test]
    fn test_quick_decompose_defaults() {
        let goal = "Create parallel background task runner to process files";
        let mut report = SpecAligner::analyze_goal(goal, None);

        assert!(report.has_ambiguity);
        let constraints = SpecAligner::quick_decompose_defaults(&mut report);

        assert!(!constraints.is_empty());
        assert!(constraints.iter().any(|c| c.contains("Tokio Async Tasks")));
    }
}
