//! MCP Client Manager, Sandboxed Plugin Supervisor, Progressive-Disclosure Skills, and 13-Stage Hooks.
use aegis_core::{AegisError, OperationKind};
use aegis_policy::PolicyEngine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub server_id: String,
    pub enabled: bool,
    pub allowed_projects: Vec<String>,
}

pub struct McpClientManager { servers: BTreeMap<String, McpServerConfig> }
impl Default for McpClientManager { fn default() -> Self { Self { servers: BTreeMap::new() } } }
impl McpClientManager {
    pub fn invoke_tool(&self, policy: &PolicyEngine, server_id: &str, tool_name: &str) -> Result<serde_json::Value, AegisError> {
        policy.evaluate(&OperationKind::McpToolCall { server_id: server_id.to_string(), tool_name: tool_name.to_string() }, &[])?;
        Ok(serde_json::json!({ "server_id": server_id, "tool": tool_name, "status": "ok" }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPackage {
    pub skill_id: String,
    pub version: String,
    pub title: String,
    pub tags: Vec<String>,
    pub instructions_markdown: String,
    pub reference_resources: BTreeMap<String, String>,
    pub enabled: bool,
}

pub struct SkillRegistry { skills: BTreeMap<String, SkillPackage> }
impl Default for SkillRegistry { fn default() -> Self { Self { skills: BTreeMap::new() } } }
