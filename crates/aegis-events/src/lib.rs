//! Real-time Broadcast Event Bus, Heartbeat Monitor, and Stage-Aware Stuck Detection (Section 15).
use aegis_core::AgentActivityState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveActivityEvent {
    pub event_id: Uuid,
    pub sequence_no: u64,
    pub task_id: Uuid,
    pub state: AgentActivityState,
    pub current_step: Option<String>,
    pub active_tool: Option<String>,
    pub active_model: Option<String>,
    pub message: String,
    pub elapsed_ms: u64,
    pub token_used: usize,
    pub token_budget: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<LiveActivityEvent>,
    sequence: Arc<Mutex<u64>>,
    last_heartbeat: Arc<Mutex<HashMap<Uuid, (AgentActivityState, DateTime<Utc>)>>>,
    recent_history: Arc<Mutex<Vec<LiveActivityEvent>>>,
}

impl Default for EventBus { fn default() -> Self { Self::new(1024) } }

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            sequence: Arc::new(Mutex::new(0)),
            last_heartbeat: Arc::new(Mutex::new(HashMap::new())),
            recent_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<LiveActivityEvent> { self.sender.subscribe() }
    pub fn emit(
        &self, task_id: Uuid, state: AgentActivityState, current_step: Option<String>,
        active_tool: Option<String>, active_model: Option<String>, message: impl Into<String>,
        token_used: usize, token_budget: usize,
    ) -> LiveActivityEvent {
        let now = Utc::now();
        let mut seq = self.sequence.lock().unwrap();
        *seq += 1;
        let sequence_no = *seq;
        drop(seq);
        self.last_heartbeat.lock().unwrap().insert(task_id, (state, now));
        let event = LiveActivityEvent {
            event_id: Uuid::new_v4(), sequence_no, task_id, state, current_step,
            active_tool, active_model, message: message.into(), elapsed_ms: 0,
            token_used, token_budget, timestamp: now,
        };
        self.recent_history.lock().unwrap().push(event.clone());
        let _ = self.sender.send(event.clone());
        event
    }
    pub fn recent_events(&self, task_id: Option<Uuid>) -> Vec<LiveActivityEvent> {
        let hist = self.recent_history.lock().unwrap();
        match task_id {
            Some(id) => hist.iter().filter(|e| e.task_id == id).cloned().collect(),
            None => hist.clone(),
        }
    }
}
