use anyhow::{anyhow, Result};
use locus_agents::skill_manifest::{LoadedSkill, SkillManifest};
use locus_agents::skill_registry::SkillRegistry;
use locus_agents::skill_runner::SkillExecutionResult;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::types::{Message, MessageRole, Tool, ToolCall};

/// Converts a LOCUS SkillManifest into an LLM-compatible Tool definition
pub fn manifest_to_tool(manifest: &SkillManifest) -> Tool {
    Tool {
        name: manifest.id.clone(),
        description: format!("{} (LOCUS Skill: {})", manifest.description, manifest.name),
        parameters: manifest.parameters.clone(),
    }
}

/// Converts a slice of LoadedSkills into active LLM Tools (filtering only enabled ones)
pub fn skills_to_tools(skills: &[LoadedSkill]) -> Vec<Tool> {
    skills
        .iter()
        .filter(|s| s.manifest.enabled && s.is_valid)
        .map(|s| manifest_to_tool(&s.manifest))
        .collect()
}

/// Bridge orchestrating LLM tool calling dispatch and skill execution
pub struct SkillBridge {
    registry: Arc<SkillRegistry>,
}

impl SkillBridge {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }

    /// Access the underlying skill registry
    pub fn registry(&self) -> Arc<SkillRegistry> {
        self.registry.clone()
    }

    /// Returns list of all active tools available for LLM injection
    pub fn get_available_tools(&self) -> Vec<Tool> {
        let skills = self.registry.list_skills();
        skills_to_tools(&skills)
    }

    /// Executes a ToolCall emitted by the model and formats the result as a Tool response Message
    pub async fn execute_tool_call(&self, call: &ToolCall) -> Result<Message> {
        let skill_id = &call.function.name;
        let args = &call.function.arguments;

        debug!("Executing LLM ToolCall: {} with args: {:?}", skill_id, args);

        let exec_result = self.registry.execute_skill(skill_id, args).await;

        let content = match exec_result {
            Ok(res) => {
                if res.success {
                    if let Some(json_val) = res.parsed_json {
                        serde_json::to_string_pretty(&json_val)
                            .unwrap_or_else(|_| res.stdout)
                    } else if !res.stdout.trim().is_empty() {
                        res.stdout
                    } else {
                        serde_json::json!({
                            "status": "success",
                            "message": "Skill executed successfully with no output"
                        })
                        .to_string()
                    }
                } else {
                    let err_msg = res.error.unwrap_or(res.stderr);
                    serde_json::json!({
                        "status": "error",
                        "error": err_msg,
                        "exit_code": res.exit_code,
                        "is_timeout": res.is_timeout
                    })
                    .to_string()
                }
            }
            Err(e) => serde_json::json!({
                "status": "error",
                "error": format!("Failed to dispatch skill: {}", e)
            })
            .to_string(),
        };

        Ok(Message {
            role: MessageRole::Tool,
            content,
            images: None,
            tool_calls: None,
            tool_call_id: Some(call.id.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_agents::skill_manifest::{SkillPermissions, SkillRuntime};
    use crate::types::ToolFunction;

    #[test]
    fn test_manifest_to_tool_conversion() {
        let manifest = SkillManifest {
            id: "disk_usage".to_string(),
            name: "Disk Usage".to_string(),
            version: "1.0.0".to_string(),
            description: "Reports free disk space".to_string(),
            author: None,
            runtime: SkillRuntime::Script,
            entrypoint: "disk.py".to_string(),
            permissions: SkillPermissions::default(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "drive": { "type": "string" }
                },
                "required": ["drive"]
            }),
            enabled: true,
            timeout_seconds: 10,
        };

        let tool = manifest_to_tool(&manifest);
        assert_eq!(tool.name, "disk_usage");
        assert!(tool.description.contains("Reports free disk space"));
        assert_eq!(tool.parameters["type"], "object");
        assert!(tool.parameters["required"].as_array().unwrap().contains(&serde_json::json!("drive")));
    }

    #[tokio::test]
    async fn test_tool_call_dispatch() {
        let registry = Arc::new(SkillRegistry::new(None));
        let bridge = SkillBridge::new(registry);

        let call = ToolCall {
            id: "call_abc123".to_string(),
            function: ToolFunction {
                name: "non_existent_skill".to_string(),
                arguments: serde_json::json!({ "query": "hello" }),
            },
        };

        let tool_response = bridge.execute_tool_call(&call).await.unwrap();
        assert_eq!(tool_response.role, MessageRole::Tool);
        assert_eq!(tool_response.tool_call_id.as_deref(), Some("call_abc123"));
        assert!(tool_response.content.contains("error"));
    }
}
