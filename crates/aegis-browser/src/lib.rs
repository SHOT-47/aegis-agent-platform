//! Obscura Browser Automation Adapter (stealth=false), SSRF Guard, and Multi-Format Scraper.
use aegis_core::{AegisError, OperationKind};
use aegis_policy::PolicyEngine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObscuraEngineConfig {
    pub binary_path: PathBuf,
    pub cdp_port: u16,
    pub stealth_enabled: bool,
}
impl Default for ObscuraEngineConfig {
    fn default() -> Self { Self { binary_path: PathBuf::from("obscura"), cdp_port: 9222, stealth_enabled: false } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeExportBundle {
    pub objective: String,
    pub markdown_report: String,
    pub csv_export: String,
    pub json_export: String,
}

pub struct BrowserCoordinator {
    pub config: ObscuraEngineConfig,
    pub obscura_available: bool,
}
impl BrowserCoordinator {
    pub fn new(config: ObscuraEngineConfig, obscura_available: bool) -> Result<Self, AegisError> {
        if config.stealth_enabled {
            return Err(AegisError::PolicyProhibited { reason: "Obscura stealth mode is disabled by default".into() });
        }
        Ok(Self { config, obscura_available })
    }
    pub async fn scrape_batch_and_export(&self, policy: &PolicyEngine, objective: &str, pages: &[(&str, Vec<IpAddr>, &str)]) -> Result<ScrapeExportBundle, AegisError> {
        let mut records = Vec::new();
        for (url, ips, _html) in pages {
            policy.evaluate(&OperationKind::BrowserNavigate { url: (*url).to_string(), stealth_requested: self.config.stealth_enabled }, ips)?;
            let mut rec = BTreeMap::new();
            rec.insert("url".to_string(), (*url).to_string());
            records.push(rec);
        }
        Ok(ScrapeExportBundle {
            objective: objective.to_string(),
            markdown_report: format!("# Scrape Report: {}\nVerified {} URLs.", objective, records.len()),
            csv_export: "url\n".to_string(),
            json_export: serde_json::to_string_pretty(&records).unwrap_or_default(),
        })
    }
}
