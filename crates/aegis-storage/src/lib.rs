//! Persistent SQLite WAL storage, crash-recovery checkpoints, secret redactor, and credential vault.
use aegis_core::{AegisError, AutonomousTask, CommandApprovalRequest};
use chrono::{DateTime, Utc};
use regex::Regex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct SecretRedactor {
    patterns: Vec<(String, Regex)>,
    custom_secrets: Vec<(String, String)>,
}

impl Default for SecretRedactor { fn default() -> Self { Self::new() } }

impl SecretRedactor {
    pub fn new() -> Self {
        let patterns = vec![
            ("OPENAI_KEY".to_string(), Regex::new(r"sk-[A-Za-z0-9_\-]{20,}").unwrap()),
            ("GITHUB_TOKEN".to_string(), Regex::new(r"gh[pousr]_[A-Za-z0-9]{20,}").unwrap()),
            ("AWS_ACCESS_KEY".to_string(), Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()),
            ("BEARER_TOKEN".to_string(), Regex::new(r"Bearer\s+[A-Za-z0-9\-._~+/]+=*").unwrap()),
            ("PRIVATE_KEY".to_string(), Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----").unwrap()),
        ];
        Self { patterns, custom_secrets: Vec::new() }
    }
    pub fn register_secret(&mut self, label: impl Into<String>, value: impl Into<String>) {
        let v = value.into();
        if v.len() >= 6 { self.custom_secrets.push((label.into(), v)); }
    }
    pub fn redact(&self, input: &str) -> String {
        let mut out = input.to_string();
        for (label, secret) in &self.custom_secrets {
            if out.contains(secret) { out = out.replace(secret, &format!("[REDACTED:{}]", label)); }
        }
        for (label, re) in &self.patterns {
            out = re.replace_all(&out, format!("[REDACTED:{}]", label)).into_owned();
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub checkpoint_id: Uuid,
    pub task_id: Uuid,
    pub step_index: usize,
    pub active_step_id: Option<String>,
    pub state_json: String,
    pub context_summary: String,
    pub modified_files: Vec<PathBuf>,
    pub passed_tests: Vec<String>,
    pub failed_tests: Vec<String>,
    pub approved_command_hashes: Vec<String>,
    pub interrupted_operation: Option<String>,
    pub next_safe_action: String,
    pub created_at: DateTime<Utc>,
}

pub struct CredentialVault {
    project_grants: HashMap<(String, String), String>,
}

impl Default for CredentialVault { fn default() -> Self { Self { project_grants: HashMap::new() } } }

impl CredentialVault {
    pub fn store_secret(&mut self, key: &str, raw_value: &str, allowed_projects: Vec<String>) {
        let mut hasher = Sha256::new();
        hasher.update(raw_value.as_bytes());
        let digest = hex::encode(hasher.finalize());
        for proj in allowed_projects {
            self.project_grants.insert((proj, key.to_string()), digest.clone());
        }
    }
    pub fn is_authorized(&self, project_id: &str, key: &str) -> bool {
        self.project_grants.contains_key(&(project_id.to_string(), key.to_string()))
    }
}

#[derive(Clone)]
pub struct StorageEngine {
    conn: Arc<Mutex<Connection>>,
    redactor: Arc<Mutex<SecretRedactor>>,
}

impl StorageEngine {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, AegisError> {
        let conn = Connection::open(db_path.as_ref()).map_err(|e| AegisError::Storage(e.to_string()))?;
        let engine = Self { conn: Arc::new(Mutex::new(conn)), redactor: Arc::new(Mutex::new(SecretRedactor::new())) };
        engine.init_schema()?;
        Ok(engine)
    }
    pub fn open_in_memory() -> Result<Self, AegisError> {
        let conn = Connection::open_in_memory().map_err(|e| AegisError::Storage(e.to_string()))?;
        let engine = Self { conn: Arc::new(Mutex::new(conn)), redactor: Arc::new(Mutex::new(SecretRedactor::new())) };
        engine.init_schema()?;
        Ok(engine)
    }
    fn init_schema(&self) -> Result<(), AegisError> {
        let conn = self.conn.lock().map_err(|e| AegisError::Storage(e.to_string()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS tasks (task_id TEXT PRIMARY KEY, project_id TEXT NOT NULL, state TEXT NOT NULL, objective TEXT NOT NULL, payload_json TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS checkpoints (checkpoint_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, step_index INTEGER NOT NULL, payload_json TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS approvals (request_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, command_sha256 TEXT NOT NULL, status TEXT NOT NULL, payload_json TEXT NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS audit_events (event_id TEXT PRIMARY KEY, task_id TEXT, category TEXT NOT NULL, summary TEXT NOT NULL, payload_json TEXT NOT NULL, created_at TEXT NOT NULL);"
        ).map_err(|e| AegisError::Storage(e.to_string()))?;
        Ok(())
    }
    pub fn save_task(&self, task: &AutonomousTask) -> Result<(), AegisError> {
        let raw_json = serde_json::to_string(task).map_err(|e| AegisError::Storage(e.to_string()))?;
        let safe_json = self.redactor.lock().unwrap().redact(&raw_json);
        let conn = self.conn.lock().map_err(|e| AegisError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO tasks (task_id, project_id, state, objective, payload_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(task_id) DO UPDATE SET state=excluded.state, payload_json=excluded.payload_json, updated_at=excluded.updated_at",
            params![task.task_id.to_string(), task.project_id, format!("{:?}", task.state), task.objective, safe_json, task.updated_at.to_rfc3339()],
        ).map_err(|e| AegisError::Storage(e.to_string()))?;
        Ok(())
    }
    pub fn save_checkpoint(&self, cp: &TaskCheckpoint) -> Result<(), AegisError> {
        let raw_json = serde_json::to_string(cp).map_err(|e| AegisError::Storage(e.to_string()))?;
        let safe_json = self.redactor.lock().unwrap().redact(&raw_json);
        let conn = self.conn.lock().map_err(|e| AegisError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO checkpoints (checkpoint_id, task_id, step_index, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![cp.checkpoint_id.to_string(), cp.task_id.to_string(), cp.step_index as i64, safe_json, cp.created_at.to_rfc3339()],
        ).map_err(|e| AegisError::Storage(e.to_string()))?;
        Ok(())
    }
    pub fn save_approval(&self, req: &CommandApprovalRequest) -> Result<(), AegisError> {
        let raw_json = serde_json::to_string(req).map_err(|e| AegisError::Storage(e.to_string()))?;
        let safe_json = self.redactor.lock().unwrap().redact(&raw_json);
        let conn = self.conn.lock().map_err(|e| AegisError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO approvals (request_id, task_id, command_sha256, status, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![req.request_id.to_string(), req.task_id.to_string(), req.command_sha256, format!("{:?}", req.status), safe_json, req.created_at.to_rfc3339()],
        ).map_err(|e| AegisError::Storage(e.to_string()))?;
        Ok(())
    }
}
