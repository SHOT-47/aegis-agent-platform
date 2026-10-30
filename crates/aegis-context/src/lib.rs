//! Continuous Token Cleanup Loop (Section 5) and Token Assist Engine (Section 6).
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedAnchorKind {
    OriginalUserRequirement, CurrentTaskObjective, ExplicitUserDecision,
    ActiveConstraintOrPermission, CurrentExecutionPlan, ImplementationDecision,
    KnownBlocker, FailedAttemptDoNotRepeat, VerificationEvidence,
    RelevantCodeReference, UnresolvedQuestion, RollbackInformation, SecurityWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub item_id: Uuid,
    pub role: String,
    pub content: String,
    pub token_count: usize,
    pub protected_kind: Option<ProtectedAnchorKind>,
    pub source_citation: Option<String>,
    pub step_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub content_sha256: String,
}

pub struct SubwordBpeTokenizer;

impl SubwordBpeTokenizer {
    pub fn count_tokens(text: &str) -> usize {
        text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .map(|w| (w.len() / 4).max(1))
            .sum::<usize>()
            .max(1)
    }
}

pub struct ContinuousTokenCleanupLoop {
    pub max_window_tokens: usize,
    pub soft_watermark_ratio: f64,
    active_items: Vec<ContextItem>,
    in_flight_model_request: bool,
}

impl ContinuousTokenCleanupLoop {
    pub fn new(max_window_tokens: usize) -> Self {
        Self { max_window_tokens, soft_watermark_ratio: 0.72, active_items: Vec::new(), in_flight_model_request: false }
    }
    pub fn push_item(
        &mut self, role: impl Into<String>, content: impl Into<String>,
        protected_kind: Option<ProtectedAnchorKind>, source_citation: Option<String>, step_id: Option<String>,
    ) -> ContextItem {
        let c = content.into();
        let mut hasher = Sha256::new();
        hasher.update(c.as_bytes());
        let item = ContextItem {
            item_id: Uuid::new_v4(), role: role.into(), content: c.clone(),
            token_count: SubwordBpeTokenizer::count_tokens(&c), protected_kind,
            source_citation, step_id, created_at: Utc::now(),
            content_sha256: hex::encode(hasher.finalize()),
        };
        self.active_items.push(item.clone());
        item
    }
    pub fn total_active_tokens(&self) -> usize { self.active_items.iter().map(|i| i.token_count).sum() }
    pub fn compact_if_needed(&mut self, _task_id: Uuid, _query: &str, _objective: &str) -> Option<usize> {
        if self.in_flight_model_request { return None; }
        let soft_limit = (self.max_window_tokens as f64 * self.soft_watermark_ratio) as usize;
        if self.total_active_tokens() < soft_limit { return None; }
        let mut seen = HashSet::new();
        self.active_items.retain(|i| i.protected_kind.is_some() || seen.insert(i.content_sha256.clone()));
        Some(self.total_active_tokens())
    }
}
