//! Unified CLI & Daemon Entrypoint (aegis).
pub mod server;
pub mod tui;

use aegis_browser::{BrowserCoordinator, ObscuraEngineConfig};
use aegis_events::EventBus;
use aegis_indexing::ProjectIntelligenceIndex;
use aegis_models::{LayaDecisionAdapter, LocalModelManager, ModelRouter, TaskModelRole};
use aegis_policy::{PolicyEngine, WorkspacePolicyConfig};
use aegis_storage::StorageEngine;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "aegis", version)]
pub struct AegisCli {
    #[arg(long, default_value = ".")]
    pub workspace: PathBuf,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Serve { #[arg(long, default_value = "127.0.0.1:18420")] bind: SocketAddr },
    Tui,
    Index { #[arg(long)] query: Option<String> },
    Models { #[arg(long)] dir: Vec<PathBuf> },
    Scrape { #[arg(long)] url: String, #[arg(long, default_value = "Extract docs")] objective: String },
    Doctor,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = AegisCli::parse();
    let workspace = std::fs::canonicalize(&cli.workspace).unwrap_or(cli.workspace.clone());
    match cli.command {
        Commands::Serve { bind } => {
            let storage = StorageEngine::open_in_memory()?;
            let policy = PolicyEngine::new(WorkspacePolicyConfig::secure_default(workspace));
            let events = EventBus::new(1024);
            let state = server::ApiServerState {
                session_token: Uuid::new_v4().to_string(),
                csrf_nonce: Uuid::new_v4().to_string(),
                storage, policy: Arc::new(Mutex::new(policy)), events,
                tasks: Arc::new(Mutex::new(Vec::new())),
                pending_approvals: Arc::new(Mutex::new(Vec::new())),
            };
            server::run_localhost_server(state, bind).await?;
        }
        Commands::Tui => {
            let vm = tui::TerminalDashboardViewModel { active_task: None, recent_events: vec![], pending_approval: None, status_banner: "Connected".into() };
            println!("{}", vm.render_ascii_snapshot());
        }
        Commands::Index { query } => {
            let mut idx = ProjectIntelligenceIndex::new(&workspace);
            println!("{}", serde_json::to_string_pretty(&idx.sync_incremental())?);
            if let Some(q) = query { println!("{}", serde_json::to_string_pretty(&idx.search(&q, 10))?); }
        }
        Commands::Models { dir } => {
            let mut mgr = LocalModelManager::new(dir);
            println!("{}", serde_json::to_string_pretty(&mgr.discover_models())?);
        }
        Commands::Scrape { url, objective } => {
            let policy = PolicyEngine::new(WorkspacePolicyConfig::secure_default(workspace));
            let coord = BrowserCoordinator::new(ObscuraEngineConfig::default(), true)?;
            let bundle = coord.scrape_batch_and_export(&policy, &objective, &[(&url, vec!["93.184.216.34".parse().unwrap()], "<html></html>")]).await?;
            println!("{}", bundle.markdown_report);
        }
        Commands::Doctor => {
            let router = ModelRouter::default();
            let laya = LayaDecisionAdapter::default();
            let (m, b, _) = router.route_or_request_fallback(Uuid::new_v4(), TaskModelRole::MainCodingAgent, true, "openai")?;
            println!("DOCTOR_OK | model={} | backend={:?} | laya_conf={}", m, b, laya.min_confidence_threshold);
        }
    }
    Ok(())
}
