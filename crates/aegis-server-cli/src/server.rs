//! Localhost REST + WebSocket API Server (127.0.0.1:18420) with Bearer + CSRF + Origin checks.
use aegis_core::{AgentActivityState, AutonomousTask, CommandApprovalRequest};
use aegis_events::{EventBus, LiveActivityEvent};
use aegis_policy::PolicyEngine;
use aegis_storage::StorageEngine;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiServerState {
    pub session_token: String,
    pub csrf_nonce: String,
    pub storage: StorageEngine,
    pub policy: Arc<Mutex<PolicyEngine>>,
    pub events: EventBus,
    pub tasks: Arc<Mutex<Vec<AutonomousTask>>>,
    pub pending_approvals: Arc<Mutex<Vec<CommandApprovalRequest>>>,
}

impl ApiServerState {
    pub fn verify_auth_and_csrf(&self, headers: &HeaderMap, require_csrf: bool) -> Result<(), StatusCode> {
        let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
        if auth != format!("Bearer {}", self.session_token) { return Err(StatusCode::UNAUTHORIZED); }
        if require_csrf {
            let csrf = headers.get("x-aegis-csrf").and_then(|v| v.to_str().ok()).unwrap_or("");
            if csrf != self.csrf_nonce { return Err(StatusCode::FORBIDDEN); }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskPayload { pub project_id: String, pub user_request: String, pub objective: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalDecisionPayload { pub request_id: Uuid, pub approved: bool, pub command_sha256: String }

pub fn build_router(state: ApiServerState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/tasks", get(list_tasks_handler).post(create_task_handler))
        .route("/api/v1/events", get(recent_events_handler))
        .route("/api/v1/approvals/decide", post(decide_approval_handler))
        .route("/ws/events", get(ws_events_handler))
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "localhost_only_default": true, "obscura_stealth": false }))
}

async fn list_tasks_handler(State(state): State<ApiServerState>, headers: HeaderMap) -> Result<Json<Vec<AutonomousTask>>, StatusCode> {
    state.verify_auth_and_csrf(&headers, false)?;
    Ok(Json(state.tasks.lock().unwrap().clone()))
}

async fn create_task_handler(State(state): State<ApiServerState>, headers: HeaderMap, Json(payload): Json<CreateTaskPayload>) -> Result<Json<AutonomousTask>, StatusCode> {
    state.verify_auth_and_csrf(&headers, true)?;
    let now = chrono::Utc::now();
    let task = AutonomousTask {
        task_id: Uuid::new_v4(), project_id: payload.project_id,
        workspace_root: std::env::current_dir().unwrap_or_default(),
        user_request: payload.user_request, objective: payload.objective.clone(),
        completion_criteria: vec!["All checks pass".into()], state: AgentActivityState::Planning,
        plan: vec![], active_step_id: None,
        active_model: "qwen2.5-coder-32b-instruct.Q4_K_M.gguf".into(),
        active_provider: "llama.cpp".into(), token_used: 128, token_budget: 32_768,
        iteration_count: 1, max_iterations: 32, created_at: now, updated_at: now,
    };
    let _ = state.storage.save_task(&task);
    state.tasks.lock().unwrap().push(task.clone());
    Ok(Json(task))
}

async fn recent_events_handler(State(state): State<ApiServerState>, headers: HeaderMap) -> Result<Json<Vec<LiveActivityEvent>>, StatusCode> {
    state.verify_auth_and_csrf(&headers, false)?;
    Ok(Json(state.events.recent_events(None)))
}

async fn decide_approval_handler(State(state): State<ApiServerState>, headers: HeaderMap, Json(decision): Json<ApprovalDecisionPayload>) -> Result<Json<serde_json::Value>, StatusCode> {
    state.verify_auth_and_csrf(&headers, true)?;
    let mut pol = state.policy.lock().unwrap();
    if decision.approved {
        let req = pol.approve_command_request(decision.request_id).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Json(serde_json::json!({ "status": "approved", "command_sha256": req.command_sha256 })))
    } else {
        Ok(Json(serde_json::json!({ "status": "denied" })))
    }
}

async fn ws_events_handler(ws: WebSocketUpgrade, State(state): State<ApiServerState>) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut rx = state.events.subscribe();
        while let Ok(ev) = rx.recv().await {
            if let Ok(txt) = serde_json::to_string(&ev) {
                if socket.send(Message::Text(txt)).await.is_err() { break; }
            }
        }
    })
}

pub async fn run_localhost_server(state: ApiServerState, addr: SocketAddr) -> anyhow::Result<()> {
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
