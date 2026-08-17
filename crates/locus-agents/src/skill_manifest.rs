use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Target execution runtime for a LOCUS skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillRuntime {
    Wasm,
    Script,
}

impl Default for SkillRuntime {
    fn default() -> Self {
        Self::Script
    }
}

/// Security permissions declared by a skill
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SkillPermissions {
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default)]
    pub allow_fs_read: bool,
    #[serde(default)]
    pub allow_fs_write: bool,
    #[serde(default)]
    pub env_whitelist: Vec<String>,
}

/// Skill location origin
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLocation {
    Workspace(PathBuf),
    Global(PathBuf),
}

/// Complete definition of a LOCUS skill manifest (skill.yaml / skill.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub runtime: SkillRuntime,
    pub entrypoint: String,
    #[serde(default)]
    pub permissions: SkillPermissions,
    /// JSON Schema object describing the tool parameters
    #[serde(default = "default_parameters_schema")]
    pub parameters: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_parameters_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}

impl SkillManifest {
    /// Parses a skill manifest file from either .yaml, .yml, or .json
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read skill manifest file at {:?}", path))?;

        if path.extension().map_or(false, |ext| ext == "json") {
            Self::from_str_json(&content)
        } else {
            Self::from_str_yaml(&content)
        }
    }

    /// Deserializes a manifest from YAML format
    pub fn from_str_yaml(content: &str) -> Result<Self> {
        let manifest: SkillManifest = serde_yaml::from_str(content)
            .context("Failed to deserialize YAML skill manifest")?;
        manifest.sanitize_and_validate()?;
        Ok(manifest)
    }

    /// Deserializes a manifest from JSON format
    pub fn from_str_json(content: &str) -> Result<Self> {
        let manifest: SkillManifest = serde_json::from_str(content)
            .context("Failed to deserialize JSON skill manifest")?;
        manifest.sanitize_and_validate()?;
        Ok(manifest)
    }

    fn sanitize_and_validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(anyhow!("Skill ID cannot be empty"));
        }
        if self.name.trim().is_empty() {
            return Err(anyhow!("Skill name cannot be empty"));
        }
        if self.entrypoint.trim().is_empty() {
            return Err(anyhow!("Skill entrypoint cannot be empty"));
        }
        Ok(())
    }

    /// Validates an input JSON payload against the manifest's JSON Schema parameter definitions.
    /// Returns Ok(()) if valid, or Err(Vec<String>) containing all validation error descriptions.
    pub fn validate_input(&self, args: &serde_json::Value) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 1. Ensure root input is an object
        let args_obj = match args.as_object() {
            Some(obj) => obj,
            None => {
                errors.push("Expected root arguments to be a JSON Object ({...})".to_string());
                return Err(errors);
            }
        };

        // 2. Check required parameters
        if let Some(required_list) = self.parameters.get("required").and_then(|r| r.as_array()) {
            for req in required_list {
                if let Some(req_str) = req.as_str() {
                    if !args_obj.contains_key(req_str) || args_obj[req_str].is_null() {
                        errors.push(format!("Missing required parameter: '{}'", req_str));
                    }
                }
            }
        }

        // 3. Check property types if declared
        if let Some(props) = self.parameters.get("properties").and_then(|p| p.as_object()) {
            for (key, val) in args_obj {
                if let Some(prop_schema) = props.get(key) {
                    if let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                        let is_type_match = match expected_type {
                            "string" => val.is_string(),
                            "number" => val.is_number(),
                            "integer" => val.is_i64() || val.is_u64(),
                            "boolean" => val.is_boolean(),
                            "array" => val.is_array(),
                            "object" => val.is_object(),
                            _ => true,
                        };

                        if !is_type_match {
                            errors.push(format!(
                                "Parameter '{}' has invalid type. Expected '{}', received {:?}",
                                key, expected_type, val
                            ));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// In-memory representation of a loaded skill discovered on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedSkill {
    pub manifest: SkillManifest,
    pub dir_path: PathBuf,
    pub entrypoint_path: PathBuf,
    pub location: SkillLocation,
    pub is_valid: bool,
    pub load_error: Option<String>,
}

impl LoadedSkill {
    pub fn new(manifest: SkillManifest, dir_path: PathBuf, location: SkillLocation) -> Self {
        let entrypoint_path = dir_path.join(&manifest.entrypoint);
        let exists = entrypoint_path.exists();
        let is_valid = exists;
        let load_error = if !exists {
            Some(format!("Entrypoint file does not exist: {:?}", entrypoint_path))
        } else {
            None
        };

        Self {
            manifest,
            dir_path,
            entrypoint_path,
            location,
            is_valid,
            load_error,
        }
    }
}

/// Generates a complete starter template for a new skill (Manifest + Code)
pub fn generate_skill_boilerplate(
    id: &str,
    name: &str,
    runtime: SkillRuntime,
    language: &str,
    description: &str,
) -> (SkillManifest, String) {
    let safe_id = id.trim().to_lowercase().replace(' ', "_");
    let safe_name = if name.trim().is_empty() {
        id
    } else {
        name.trim()
    };
    let desc = if description.trim().is_empty() {
        "Custom LOCUS agent skill capability"
    } else {
        description.trim()
    };

    let (entrypoint, code) = match (runtime, language.to_lowercase().as_str()) {
        (SkillRuntime::Wasm, _) => (
            "main.wasm".to_string(),
            "// Compile your Rust/C/Go code to WASM targeting wasm32-wasip1".to_string(),
        ),
        (SkillRuntime::Script, "python" | "py") => (
            "main.py".to_string(),
            r#"#!/usr/bin/env python3
import sys
import json
import os

def main():
    # Read input from LOCUS stdin or LOCUS_INPUT_JSON env var
    input_str = os.environ.get("LOCUS_INPUT_JSON")
    if not input_str:
        input_str = sys.stdin.read().strip() or "{}"
    
    try:
        args = json.loads(input_str)
    except Exception as e:
        args = {}

    query = args.get("query", "Default LOCUS query")
    
    # Process skill logic
    result = {
        "status": "success",
        "message": f"Processed query: {query}",
        "processed_by": "LOCUS Python Skill Runner"
    }
    
    # Print JSON result to stdout
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()
"#.to_string(),
        ),
        (SkillRuntime::Script, "javascript" | "js" | "node") => (
            "index.js".to_string(),
            r#"#!/usr/bin/env node
const fs = require('fs');

function main() {
    let inputStr = process.env.LOCUS_INPUT_JSON;
    if (!inputStr) {
        try {
            inputStr = fs.readFileSync(0, 'utf-8').trim() || '{}';
        } catch {
            inputStr = '{}';
        }
    }

    const args = JSON.parse(inputStr || '{}');
    const query = args.query || 'Default LOCUS query';

    const result = {
        status: 'success',
        message: `Processed query: ${query}`,
        processed_by: 'LOCUS Node.js Skill Runner'
    };

    console.log(JSON.stringify(result, null, 2));
}

main();
"#.to_string(),
        ),
        (SkillRuntime::Script, "powershell" | "ps1") => (
            "script.ps1".to_string(),
            r#"$inputJson = $env:LOCUS_INPUT_JSON
if (-not $inputJson) {
    $inputJson = [Console]::In.ReadToEnd()
}

$argsObj = @{}
if ($inputJson) {
    try {
        $argsObj = $inputJson | ConvertFrom-Json
    } catch {}
}

$query = if ($argsObj.query) { $argsObj.query } else { "Default LOCUS query" }

$result = [PSCustomObject]@{
    status = "success"
    message = "Processed query: $query"
    processed_by = "LOCUS PowerShell Skill Runner"
}

$result | ConvertTo-Json -Depth 4
"#.to_string(),
        ),
        (SkillRuntime::Script, _) => (
            "run.sh".to_string(),
            r#"#!/bin/bash
INPUT="${LOCUS_INPUT_JSON:-$(cat)}"
echo "{\"status\": \"success\", \"raw_input\": \"$INPUT\", \"processed_by\": \"LOCUS Shell Skill Runner\"}"
"#.to_string(),
        ),
    };

    let manifest = SkillManifest {
        id: safe_id,
        name: safe_name.to_string(),
        version: "0.1.0".to_string(),
        description: desc.to_string(),
        author: Some("LOCUS User".to_string()),
        runtime,
        entrypoint,
        permissions: SkillPermissions {
            allow_network: false,
            allow_fs_read: true,
            allow_fs_write: false,
            env_whitelist: vec![],
        },
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Input text or command parameter to execute"
                }
            },
            "required": ["query"]
        }),
        enabled: true,
        timeout_seconds: 30,
    };

    (manifest, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_manifest_yaml() {
        let yaml = r#"
id: git_status_checker
name: Git Status Checker
version: 1.0.0
description: Inspects active repository working tree
runtime: script
entrypoint: git_status.py
permissions:
  allow_fs_read: true
  allow_fs_write: false
  allow_network: false
parameters:
  type: object
  properties:
    path:
      type: string
      description: Target git directory path
  required:
    - path
enabled: true
timeout_seconds: 15
"#;

        let manifest = SkillManifest::from_str_yaml(yaml).unwrap();
        assert_eq!(manifest.id, "git_status_checker");
        assert_eq!(manifest.runtime, SkillRuntime::Script);
        assert_eq!(manifest.timeout_seconds, 15);
        assert!(manifest.permissions.allow_fs_read);
        assert!(!manifest.permissions.allow_network);

        // Validation pass
        let valid_args = serde_json::json!({ "path": "D:/LOCUS" });
        assert!(manifest.validate_input(&valid_args).is_ok());

        // Missing required field
        let invalid_args = serde_json::json!({ "other": 123 });
        let err = manifest.validate_input(&invalid_args).unwrap_err();
        assert!(err[0].contains("Missing required parameter: 'path'"));

        // Wrong type
        let wrong_type_args = serde_json::json!({ "path": 42 });
        let err2 = manifest.validate_input(&wrong_type_args).unwrap_err();
        assert!(err2[0].contains("invalid type"));
    }

    #[test]
    fn test_generate_boilerplate_python() {
        let (manifest, code) = generate_skill_boilerplate(
            "custom_analyzer",
            "Custom Analyzer",
            SkillRuntime::Script,
            "python",
            "Analyzes code files",
        );

        assert_eq!(manifest.id, "custom_analyzer");
        assert_eq!(manifest.entrypoint, "main.py");
        assert!(code.contains("def main():"));
    }
}
