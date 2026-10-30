//! Core domain types, 22-state activity machine, plans, approvals, and error taxonomy.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivityState {
    Initializing, UnderstandingRequest, Planning, SearchingProject, Indexing,
    RetrievingContext, PreparingModelRequest, RunningInference, ExecutingTool,
    RunningCommand, WaitingForApproval, BrowsingWebsite, ScrapingPages,
    DelegatingAgent, RunningTests, VerifyingChanges, RecoveringFromError,
    Paused, Blocked, Completed, Failed, Canceled,
}

impl AgentActivityState {
    pub fn default_stage_timeout_secs(&self) -> u64 {
        match self {
            Self::Initializing | Self::PreparingModelRequest => 30,
            Self::UnderstandingRequest | Self::Planning => 90,
            Self::SearchingProject | Self::Indexing | Self::RetrievingContext => 120,
            Self::RunningInference => 180,
            Self::ExecutingTool | Self::RunningCommand => 300,
            Self::WaitingForApproval | Self::Paused | Self::Blocked => u64::MAX,
            Self::BrowsingWebsite | Self::ScrapingPages | Self::RecoveringFromError => 120,
            Self::DelegatingAgent => 240,
            Self::RunningTests | Self::VerifyingChanges => 600,
            Self::Completed | Self::Failed | Self::Canceled => 0,
        }
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
    pub fn is_legitimately_waiting(&self) -> bool {
        matches!(self, Self::WaitingForApproval | Self::Paused | Self::Blocked)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier { Low, Elevated, Prohibited }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    ReadFile { path: PathBuf },
    WriteFile { path: PathBuf, bytes_len: usize },
    DeleteFile { path: PathBuf },
    ShellCommand { argv: Vec<String>, cwd: PathBuf, env_deltas: BTreeMap<String, String> },
    BrowserNavigate { url: String, stealth_requested: bool },
    BrowserFormSubmit { url: String, method: String },
    WebScrapeBatch { urls: Vec<String>, concurrency: usize },
    McpToolCall { server_id: String, tool_name: String },
    PluginInstall { plugin_id: String, version: String, permissions: Vec<String> },
    SkillExecute { skill_id: String },
    HookExecute { hook_id: String, stage: String },
    ModelLoadLocal { model_id: String, backend: String },
    ModelEngineBuildTensorRt { model_id: String, estimated_vram_mb: u64, estimated_disk_mb: u64 },
    ModelCloudFallback { from_local_model: String, to_cloud_provider: String },
    CredentialRead { secret_key: String, task_id: Uuid },
    SensitiveDataExport { destination: String, category: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus { Pending, Approved, Denied, Expired }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalRequest {
    pub request_id: Uuid,
    pub task_id: Uuid,
    pub exact_command: String,
    pub argv: Vec<String>,
    pub working_directory: PathBuf,
    pub target_resources: Vec<String>,
    pub reason: String,
    pub expected_result: String,
    pub potential_impact: String,
    pub relevant_risks: Vec<String>,
    pub environment_changes: BTreeMap<String, String>,
    pub rollback_information: String,
    pub command_sha256: String,
    pub created_at: DateTime<Utc>,
    pub status: ApprovalStatus,
}

impl CommandApprovalRequest {
    pub fn compute_command_hash(argv: &[String], cwd: &PathBuf, env_changes: &BTreeMap<String, String>) -> String {
        let mut hasher = Sha256::new();
        for arg in argv { hasher.update(arg.as_bytes()); hasher.update(b"\x00"); }
        hasher.update(cwd.to_string_lossy().as_bytes());
        hasher.update(b"\x00");
        for (k, v) in env_changes {
            hasher.update(k.as_bytes()); hasher.update(b"="); hasher.update(v.as_bytes()); hasher.update(b"\x00");
        }
        hex::encode(hasher.finalize())
    }
    pub fn verify_matches(&self, argv: &[String], cwd: &PathBuf, env_changes: &BTreeMap<String, String>) -> bool {
        self.status == ApprovalStatus::Approved && Self::compute_command_hash(argv, cwd, env_changes) == self.command_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus { Pending, InProgress, WaitingApproval, Verified, Blocked, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: String,
    pub title: String,
    pub description: String,
    pub done_condition: String,
    pub assigned_specialist: Option<String>,
    pub depends_on: Vec<String>,
    pub status: PlanStepStatus,
    pub retries: u32,
    pub verification_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousTask {
    pub task_id: Uuid,
    pub project_id: String,
    pub workspace_root: PathBuf,
    pub user_request: String,
    pub objective: String,
    pub completion_criteria: Vec<String>,
    pub state: AgentActivityState,
    pub plan: Vec<PlanStep>,
    pub active_step_id: Option<String>,
    pub active_model: String,
    pub active_provider: String,
    pub token_used: usize,
    pub token_budget: usize,
    pub iteration_count: u32,
    pub max_iterations: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum AegisError {
    #[error("Workspace boundary violation: {path} outside {workspace}")]
    WorkspaceBoundaryViolation { path: String, workspace: String },
    #[error("Policy prohibited operation: {reason}")]
    PolicyProhibited { reason: String },
    #[error("Explicit user approval required ({request_id}): {reason}")]
    ApprovalRequired { request_id: Uuid, reason: String },
    #[error("SSRF or restricted network target blocked: {url} ({reason})")]
    NetworkRestricted { url: String, reason: String },
    #[error("Model runtime error ({backend}): {message}")]
    ModelRuntime { backend: String, message: String },
    #[error("Local-to-cloud fallback blocked pending approval: {from} -> {to}")]
    CloudFallbackApprovalRequired { from: String, to: String },
    #[error("Context window overflow: required {required}, available {available}")]
    ContextOverflow { required: usize, available: usize },
    #[error("Browser automation error ({engine}): {message}")]
    BrowserError { engine: String, message: String },
    #[error("Stuck operation detected in stage {stage:?} after {elapsed_secs}s")]
    OperationStuck { stage: AgentActivityState, elapsed_secs: u64 },
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Verification check failed: {0}")]
    VerificationFailed(String),
}
