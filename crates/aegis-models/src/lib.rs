//! Local LLM Runtime Adapters (llama.cpp, NVIDIA TensorRT-LLM), Laya System-1 Engine, and Router.
use aegis_core::AegisError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskModelRole {
    MainCodingAgent, PlanningAndReasoning, CodeReview, LightweightClassification,
    EmbeddingsAndRetrieval, SummarizationAndCleanup, BrowserAndWebResearch, SpecialistAgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceBackendKind { LlamaCpp, TensorRtLlm, LayaSystem1, ExternalOpenAiCompatible }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredLocalModel {
    pub model_id: String,
    pub path: PathBuf,
    pub format: String,
    pub size_bytes: u64,
}

pub struct LlamaCppAdapter;
impl LlamaCppAdapter {
    pub fn inspect_gguf_header(path: &Path) -> Result<u32, AegisError> {
        let mut f = File::open(path).map_err(|e| AegisError::ModelRuntime { backend: "llama.cpp".into(), message: e.to_string() })?;
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic).map_err(|e| AegisError::ModelRuntime { backend: "llama.cpp".into(), message: e.to_string() })?;
        if &magic[0..4] != b"GGUF" {
            return Err(AegisError::ModelRuntime { backend: "llama.cpp".into(), message: "Invalid GGUF header".into() });
        }
        Ok(u32::from_le_bytes(magic[4..8].try_into().unwrap()))
    }
}

pub struct LayaDecisionAdapter {
    pub min_confidence_threshold: f64,
    pub temperature_scale: f64,
}
impl Default for LayaDecisionAdapter {
    fn default() -> Self { Self { min_confidence_threshold: 0.68, temperature_scale: 1.35 } }
}

pub struct LocalModelManager { dirs: Vec<PathBuf> }
impl LocalModelManager {
    pub fn new(dirs: Vec<PathBuf>) -> Self { Self { dirs } }
    pub fn discover_models(&mut self) -> Vec<DiscoveredLocalModel> {
        let mut out = Vec::new();
        for d in &self.dirs {
            let Ok(entries) = fs::read_dir(d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("gguf") {
                    out.push(DiscoveredLocalModel {
                        model_id: p.file_stem().unwrap().to_string_lossy().into_owned(),
                        path: p, format: "GGUF".into(),
                        size_bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                    });
                }
            }
        }
        out
    }
}

pub struct ModelRouter {
    role_table: BTreeMap<TaskModelRole, (String, InferenceBackendKind)>,
    approved_fallbacks: HashSet<Uuid>,
}
impl Default for ModelRouter {
    fn default() -> Self {
        let mut role_table = BTreeMap::new();
        role_table.insert(TaskModelRole::MainCodingAgent, ("qwen2.5-coder-32b-instruct.Q4_K_M.gguf".into(), InferenceBackendKind::LlamaCpp));
        Self { role_table, approved_fallbacks: HashSet::new() }
    }
}
impl ModelRouter {
    pub fn route_or_request_fallback(&self, task_id: Uuid, role: TaskModelRole, local_healthy: bool, cloud_provider: &str) -> Result<(String, InferenceBackendKind, String), AegisError> {
        let (m, b) = self.role_table.get(&role).cloned().unwrap_or_else(|| ("local.gguf".into(), InferenceBackendKind::LlamaCpp));
        if local_healthy { Ok((m, b, "Local model active".into())) }
        else if self.approved_fallbacks.contains(&task_id) { Ok((cloud_provider.into(), InferenceBackendKind::ExternalOpenAiCompatible, "User approved fallback".into())) }
        else { Err(AegisError::CloudFallbackApprovalRequired { from: m, to: cloud_provider.into() }) }
    }
}
