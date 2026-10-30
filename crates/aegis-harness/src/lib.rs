//! Continuous Autonomous Harness (Section 2), 12 Specialist Agents (Section 14), and Self-Recovery (Section 20).
use aegis_context::{ContinuousTokenCleanupLoop, ProtectedAnchorKind};
use aegis_core::{AegisError, AgentActivityState, AutonomousTask, PlanStepStatus};
use aegis_events::EventBus;
use aegis_policy::PolicyEngine;
use aegis_storage::{StorageEngine, TaskCheckpoint};
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

pub struct AutonomousHarness {
    pub storage: StorageEngine,
    pub policy: PolicyEngine,
    pub events: EventBus,
    pub token_loop: ContinuousTokenCleanupLoop,
}

impl AutonomousHarness {
    pub fn new(storage: StorageEngine, policy: PolicyEngine, events: EventBus, max_tokens: usize) -> Self {
        Self { storage, policy, events, token_loop: ContinuousTokenCleanupLoop::new(max_tokens) }
    }
    pub fn initialize_task(&mut self, task: &mut AutonomousTask) -> Result<(), AegisError> {
        task.state = AgentActivityState::UnderstandingRequest;
        self.token_loop.push_item("user", &task.user_request, Some(ProtectedAnchorKind::OriginalUserRequirement), None, None);
        self.storage.save_task(task)?;
        Ok(())
    }
    pub fn advance_step(&mut self, task: &mut AutonomousTask, step_id: &str, verification_evidence: Vec<String>, modified_files: Vec<PathBuf>, passed_tests: Vec<String>) -> Result<TaskCheckpoint, AegisError> {
        for s in &mut task.plan {
            if s.step_id == step_id { s.status = PlanStepStatus::Verified; s.verification_evidence = verification_evidence.clone(); }
        }
        let cp = TaskCheckpoint {
            checkpoint_id: Uuid::new_v4(), task_id: task.task_id, step_index: task.iteration_count as usize,
            active_step_id: Some(step_id.to_string()), state_json: format!("{:?}", task.state),
            context_summary: format!("Verified {}", step_id), modified_files, passed_tests,
            failed_tests: vec![], approved_command_hashes: vec![], interrupted_operation: None,
            next_safe_action: "Continue".into(), created_at: Utc::now(),
        };
        self.storage.save_checkpoint(&cp)?;
        Ok(cp)
    }
}
