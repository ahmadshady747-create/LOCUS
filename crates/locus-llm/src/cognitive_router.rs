//! Cognitive Router & Cost-to-Power Optimizer
//!
//! Intelligently classifies tasks by cognitive load (Micro, Standard, Architectural)
//! and dynamically routes execution across available local and cloud AI models to minimize token
//! burn while maximizing reasoning depth.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveTaskComplexity {
    /// Fast routine actions: Conventional Commits, diff indentation, docstrings, regex error message parsing
    Micro,
    /// Standard engineering: function implementations, unit tests, single component edits
    Standard,
    /// Complex reasoning: DAG decomposition, Specification Alignment Gate, Adversarial QA Fuzzing, multi-crate architecture
    Architectural,
}

impl std::fmt::Display for CognitiveTaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Micro => write!(f, "Micro Task"),
            Self::Standard => write!(f, "Standard Task"),
            Self::Architectural => write!(f, "Architectural Task"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStrategy {
    /// Prioritize local models (Ollama/Llama.cpp) and free-tier ultra-fast providers (Groq/Gemini Flash)
    MaxSpeed,
    /// Default: Free/local for Micro, Balanced for Standard, High-Reasoning for Architectural (saves 75%+ tokens)
    Balanced,
    /// Directs Standard and Architectural tasks to top reasoning models (Claude 3.5 Sonnet / DeepSeek R1)
    MaxPower,
}

impl Default for BudgetStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CostTier {
    Free,
    Low,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingDecision {
    pub selected_model: String,
    pub provider: String,
    pub complexity: CognitiveTaskComplexity,
    pub cost_tier: CostTier,
    pub budget_strategy: BudgetStrategy,
    pub rationale: String,
}

pub struct CognitiveRouter;

impl CognitiveRouter {
    /// Classifies an incoming instruction prompt and context metrics into a CognitiveTaskComplexity tier
    pub fn classify_prompt(
        prompt: &str,
        file_count: usize,
        context_tokens: usize,
    ) -> CognitiveTaskComplexity {
        let p = prompt.to_lowercase();

        // 1. Check for Architectural / Multi-Phase High Reasoning signatures
        if p.contains("decompose")
            || p.contains("architecture")
            || p.contains("tradeoff")
            || p.contains("adversarial")
            || p.contains("fuzz")
            || p.contains("dag")
            || p.contains("spec align")
            || file_count >= 4
            || context_tokens > 16_000
        {
            return CognitiveTaskComplexity::Architectural;
        }

        // 2. Check for Micro Task signatures (formatting, commit synthesis, typos)
        if p.starts_with("feat(")
            || p.starts_with("fix(")
            || p.starts_with("refactor(")
            || p.contains("commit message")
            || p.contains("indentation")
            || p.contains("docstring")
            || p.contains("explain error")
            || (p.len() < 80 && !p.contains("implement") && !p.contains("write") && !p.contains("create"))
        {
            return CognitiveTaskComplexity::Micro;
        }

        // 3. Default to Standard Task
        CognitiveTaskComplexity::Standard
    }

    /// Classifies a DAG task node by its node_type and description
    pub fn classify_node(node_type: &str, description: &str) -> CognitiveTaskComplexity {
        let nt = node_type.to_lowercase();
        let desc = description.to_lowercase();

        if nt == "analysis" || desc.contains("fuzz") || desc.contains("adversarial") || desc.contains("tradeoff") {
            CognitiveTaskComplexity::Architectural
        } else if nt == "shell_command" || desc.contains("commit") || desc.contains("format") || desc.contains("lint") {
            CognitiveTaskComplexity::Micro
        } else {
            CognitiveTaskComplexity::Standard
        }
    }

    /// Dynamically routes a task based on complexity, budget strategy, and available configured providers
    pub fn route(
        complexity: CognitiveTaskComplexity,
        strategy: BudgetStrategy,
        configured_providers: &[String],
        local_models: &[String],
    ) -> RoutingDecision {
        let has_provider = |name: &str| configured_providers.iter().any(|p| p.eq_ignore_ascii_case(name));
        let has_local = !local_models.is_empty();

        match strategy {
            BudgetStrategy::MaxSpeed => {
                // Prioritize local models or Groq / Gemini Flash
                if has_local {
                    RoutingDecision {
                        selected_model: local_models[0].clone(),
                        provider: "local".to_string(),
                        complexity,
                        cost_tier: CostTier::Free,
                        budget_strategy: strategy,
                        rationale: "MaxSpeed strategy: executing on-device local model with 0 latency & 0 cost.".to_string(),
                    }
                } else if has_provider("groq") {
                    RoutingDecision {
                        selected_model: "llama-3.3-70b-versatile".to_string(),
                        provider: "groq".to_string(),
                        complexity,
                        cost_tier: CostTier::Free,
                        budget_strategy: strategy,
                        rationale: "MaxSpeed strategy: executing via ultra-fast LPUs on Groq Free Tier.".to_string(),
                    }
                } else if has_provider("gemini") {
                    RoutingDecision {
                        selected_model: "gemini-2.0-flash".to_string(),
                        provider: "gemini".to_string(),
                        complexity,
                        cost_tier: CostTier::Free,
                        budget_strategy: strategy,
                        rationale: "MaxSpeed strategy: routing to Gemini 2.0 Flash sub-second endpoint.".to_string(),
                    }
                } else {
                    Self::fallback_decision(complexity, strategy)
                }
            }

            BudgetStrategy::Balanced => {
                // Tiered Allocation:
                // Micro -> Local / Groq / Gemini Flash (0 cost)
                // Standard -> Gemini 2.0 Flash / DeepSeek Chat / Groq
                // Architectural -> DeepSeek Reasoner / Gemini 1.5 Pro / Claude Sonnet
                match complexity {
                    CognitiveTaskComplexity::Micro => {
                        if has_provider("groq") {
                            RoutingDecision {
                                selected_model: "llama-3.3-70b-versatile".to_string(),
                                provider: "groq".to_string(),
                                complexity,
                                cost_tier: CostTier::Free,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: routine micro task assigned to Groq LPU (100% token savings).".to_string(),
                            }
                        } else if has_local {
                            RoutingDecision {
                                selected_model: local_models[0].clone(),
                                provider: "local".to_string(),
                                complexity,
                                cost_tier: CostTier::Free,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: routine micro task executed locally on-device.".to_string(),
                            }
                        } else if has_provider("gemini") {
                            RoutingDecision {
                                selected_model: "gemini-2.0-flash".to_string(),
                                provider: "gemini".to_string(),
                                complexity,
                                cost_tier: CostTier::Free,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: micro task assigned to Gemini 2.0 Flash free quota.".to_string(),
                            }
                        } else {
                            Self::fallback_decision(complexity, strategy)
                        }
                    }

                    CognitiveTaskComplexity::Standard => {
                        if has_provider("gemini") {
                            RoutingDecision {
                                selected_model: "gemini-2.0-flash".to_string(),
                                provider: "gemini".to_string(),
                                complexity,
                                cost_tier: CostTier::Low,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: standard code implementation assigned to Gemini 2.0 Flash.".to_string(),
                            }
                        } else if has_provider("deepseek") {
                            RoutingDecision {
                                selected_model: "deepseek-chat".to_string(),
                                provider: "deepseek".to_string(),
                                complexity,
                                cost_tier: CostTier::Low,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: standard code implementation assigned to DeepSeek V3.".to_string(),
                            }
                        } else if has_provider("groq") {
                            RoutingDecision {
                                selected_model: "llama-3.3-70b-versatile".to_string(),
                                provider: "groq".to_string(),
                                complexity,
                                cost_tier: CostTier::Free,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: standard task routed to Groq Llama 3.3 70B.".to_string(),
                            }
                        } else {
                            Self::fallback_decision(complexity, strategy)
                        }
                    }

                    CognitiveTaskComplexity::Architectural => {
                        if has_provider("deepseek") {
                            RoutingDecision {
                                selected_model: "deepseek-reasoner".to_string(),
                                provider: "deepseek".to_string(),
                                complexity,
                                cost_tier: CostTier::Low,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: architectural reasoning assigned to DeepSeek R1 Reasoner.".to_string(),
                            }
                        } else if has_provider("anthropic") {
                            RoutingDecision {
                                selected_model: "claude-3-5-sonnet-20241022".to_string(),
                                provider: "anthropic".to_string(),
                                complexity,
                                cost_tier: CostTier::High,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: architectural reasoning assigned to Claude 3.5 Sonnet.".to_string(),
                            }
                        } else if has_provider("gemini") {
                            RoutingDecision {
                                selected_model: "gemini-1.5-pro".to_string(),
                                provider: "gemini".to_string(),
                                complexity,
                                cost_tier: CostTier::Low,
                                budget_strategy: strategy,
                                rationale: "Balanced strategy: complex architectural planning assigned to Gemini 1.5 Pro.".to_string(),
                            }
                        } else {
                            Self::fallback_decision(complexity, strategy)
                        }
                    }
                }
            }

            BudgetStrategy::MaxPower => {
                // Always select top reasoning model available
                if has_provider("anthropic") {
                    RoutingDecision {
                        selected_model: "claude-3-5-sonnet-20241022".to_string(),
                        provider: "anthropic".to_string(),
                        complexity,
                        cost_tier: CostTier::High,
                        budget_strategy: strategy,
                        rationale: "MaxPower strategy: routed to Claude 3.5 Sonnet for maximum cognitive depth.".to_string(),
                    }
                } else if has_provider("deepseek") {
                    RoutingDecision {
                        selected_model: "deepseek-reasoner".to_string(),
                        provider: "deepseek".to_string(),
                        complexity,
                        cost_tier: CostTier::Low,
                        budget_strategy: strategy,
                        rationale: "MaxPower strategy: routed to DeepSeek R1 Reasoner for full chain-of-thought.".to_string(),
                    }
                } else if has_provider("openai") {
                    RoutingDecision {
                        selected_model: "gpt-4o".to_string(),
                        provider: "openai".to_string(),
                        complexity,
                        cost_tier: CostTier::High,
                        budget_strategy: strategy,
                        rationale: "MaxPower strategy: routed to OpenAI GPT-4o.".to_string(),
                    }
                } else if has_provider("gemini") {
                    RoutingDecision {
                        selected_model: "gemini-1.5-pro".to_string(),
                        provider: "gemini".to_string(),
                        complexity,
                        cost_tier: CostTier::Low,
                        budget_strategy: strategy,
                        rationale: "MaxPower strategy: routed to Gemini 1.5 Pro high-context reasoning.".to_string(),
                    }
                } else {
                    Self::fallback_decision(complexity, strategy)
                }
            }
        }
    }

    fn fallback_decision(complexity: CognitiveTaskComplexity, strategy: BudgetStrategy) -> RoutingDecision {
        RoutingDecision {
            selected_model: "default-local".to_string(),
            provider: "local".to_string(),
            complexity,
            cost_tier: CostTier::Free,
            budget_strategy: strategy,
            rationale: "Default on-device fallback model (offline ready).".to_string(),
        }
    }

    // --- Persistence ---

    fn get_strategy_path() -> PathBuf {
        let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join(".locus").join("budget_strategy.json")
    }

    pub fn get_persisted_strategy() -> BudgetStrategy {
        let path = Self::get_strategy_path();
        if !path.exists() {
            return BudgetStrategy::default();
        }
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            BudgetStrategy::default()
        }
    }

    pub fn save_persisted_strategy(strategy: BudgetStrategy) -> Result<()> {
        let path = Self::get_strategy_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&strategy)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_micro_task() {
        let prompt = "feat(ui): add floating QA inspect badge to task cards";
        let complexity = CognitiveRouter::classify_prompt(prompt, 1, 300);
        assert_eq!(complexity, CognitiveTaskComplexity::Micro);
    }

    #[test]
    fn test_classify_architectural_task() {
        let prompt = "Decompose authentication goal into DAG and evaluate persistence tradeoffs";
        let complexity = CognitiveRouter::classify_prompt(prompt, 5, 22_000);
        assert_eq!(complexity, CognitiveTaskComplexity::Architectural);
    }

    #[test]
    fn test_classify_standard_task() {
        let prompt = "Implement user session validation function with bcrypt hashing and return Result";
        let complexity = CognitiveRouter::classify_prompt(prompt, 1, 1_200);
        assert_eq!(complexity, CognitiveTaskComplexity::Standard);
    }

    #[test]
    fn test_balanced_routing_micro_to_groq() {
        let providers = vec!["groq".to_string(), "gemini".to_string(), "deepseek".to_string()];
        let decision = CognitiveRouter::route(
            CognitiveTaskComplexity::Micro,
            BudgetStrategy::Balanced,
            &providers,
            &[],
        );

        assert_eq!(decision.provider, "groq");
        assert_eq!(decision.cost_tier, CostTier::Free);
    }

    #[test]
    fn test_balanced_routing_architectural_to_deepseek_reasoner() {
        let providers = vec!["groq".to_string(), "deepseek".to_string()];
        let decision = CognitiveRouter::route(
            CognitiveTaskComplexity::Architectural,
            BudgetStrategy::Balanced,
            &providers,
            &[],
        );

        assert_eq!(decision.provider, "deepseek");
        assert_eq!(decision.selected_model, "deepseek-reasoner");
    }

    #[test]
    fn test_max_speed_routing_prefers_local() {
        let providers = vec!["anthropic".to_string()];
        let locals = vec!["qwen2.5-coder:7b".to_string()];
        let decision = CognitiveRouter::route(
            CognitiveTaskComplexity::Standard,
            BudgetStrategy::MaxSpeed,
            &providers,
            &locals,
        );

        assert_eq!(decision.provider, "local");
        assert_eq!(decision.selected_model, "qwen2.5-coder:7b");
    }
}
