//! Centralized Tool & Risk Policy Engine, Workspace Boundary Jail, and SSRF/DNS Rebinding Guard.
use aegis_core::{AegisError, ApprovalStatus, CommandApprovalRequest, OperationKind, RiskTier};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePolicyConfig {
    pub workspace_root: PathBuf,
    pub allowed_domains: HashSet<String>,
    pub denied_domains: HashSet<String>,
    pub block_private_networks: bool,
    pub allow_obscura_stealth: bool,
    pub max_scrape_concurrency: usize,
}

impl WorkspacePolicyConfig {
    pub fn secure_default(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            allowed_domains: HashSet::new(),
            denied_domains: HashSet::from(["169.254.169.254".to_string(), "metadata.google.internal".to_string()]),
            block_private_networks: true,
            allow_obscura_stealth: false,
            max_scrape_concurrency: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub risk_tier: RiskTier,
    pub permitted: bool,
    pub approval_request_id: Option<Uuid>,
    pub reason: String,
}

pub struct PolicyEngine {
    config: WorkspacePolicyConfig,
    approved_command_hashes: HashSet<String>,
    pending_approvals: HashMap<Uuid, CommandApprovalRequest>,
}

impl PolicyEngine {
    pub fn new(config: WorkspacePolicyConfig) -> Self {
        Self { config, approved_command_hashes: HashSet::new(), pending_approvals: HashMap::new() }
    }
    pub fn is_within_workspace(&self, candidate: &Path) -> bool {
        let joined = if candidate.is_absolute() { candidate.to_path_buf() } else { self.config.workspace_root.join(candidate) };
        Self::lexical_normalize(&joined).starts_with(Self::lexical_normalize(&self.config.workspace_root))
    }
    fn lexical_normalize(path: &Path) -> PathBuf {
        let mut out = PathBuf::new();
        for comp in path.components() {
            match comp {
                Component::ParentDir => { out.pop(); }
                Component::CurDir => {}
                other => out.push(other.as_os_str()),
            }
        }
        out
    }
    pub fn is_private_or_reserved_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_broadcast() || v4.is_unspecified(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80,
        }
    }
    pub fn validate_network_target(&self, raw_url: &str, resolved_ips: &[IpAddr]) -> Result<(), AegisError> {
        let parsed = Url::parse(raw_url).map_err(|e| AegisError::NetworkRestricted { url: raw_url.to_string(), reason: e.to_string() })?;
        let host = parsed.host_str().unwrap_or("");
        if self.config.denied_domains.contains(host) || (self.config.block_private_networks && host.eq_ignore_ascii_case("localhost")) {
            return Err(AegisError::NetworkRestricted { url: raw_url.to_string(), reason: "Blocked host by SSRF/denylist policy".into() });
        }
        if self.config.block_private_networks {
            for ip in resolved_ips {
                if Self::is_private_or_reserved_ip(ip) {
                    return Err(AegisError::NetworkRestricted { url: raw_url.to_string(), reason: format!("Restricted internal IP {}", ip) });
                }
            }
        }
        Ok(())
    }
    pub fn approve_command_request(&mut self, request_id: Uuid) -> Result<CommandApprovalRequest, AegisError> {
        let req = self.pending_approvals.get_mut(&request_id).ok_or_else(|| AegisError::VerificationFailed("Unknown approval request".into()))?;
        req.status = ApprovalStatus::Approved;
        self.approved_command_hashes.insert(req.command_sha256.clone());
        Ok(req.clone())
    }
    pub fn evaluate(&self, op: &OperationKind, resolved_ips: &[IpAddr]) -> Result<PolicyDecision, AegisError> {
        match op {
            OperationKind::ReadFile { path } | OperationKind::WriteFile { path, .. } => {
                if self.is_within_workspace(path) {
                    Ok(PolicyDecision { risk_tier: RiskTier::Low, permitted: true, approval_request_id: None, reason: "Inside workspace".into() })
                } else {
                    Err(AegisError::WorkspaceBoundaryViolation { path: path.display().to_string(), workspace: self.config.workspace_root.display().to_string() })
                }
            }
            OperationKind::ShellCommand { argv, cwd, env_deltas } => {
                let hash = CommandApprovalRequest::compute_command_hash(argv, cwd, env_deltas);
                if self.approved_command_hashes.contains(&hash) {
                    Ok(PolicyDecision { risk_tier: RiskTier::Elevated, permitted: true, approval_request_id: None, reason: "Approved SHA-256".into() })
                } else {
                    Err(AegisError::ApprovalRequired { request_id: Uuid::new_v4(), reason: format!("Command requires approval (sha256={})", hash) })
                }
            }
            OperationKind::BrowserNavigate { url, stealth_requested } => {
                if *stealth_requested && !self.config.allow_obscura_stealth {
                    return Err(AegisError::PolicyProhibited { reason: "Obscura stealth mode is disabled by default".into() });
                }
                self.validate_network_target(url, resolved_ips)?;
                Ok(PolicyDecision { risk_tier: RiskTier::Low, permitted: true, approval_request_id: None, reason: "Navigation allowed".into() })
            }
            OperationKind::ModelCloudFallback { from_local_model, to_cloud_provider } => {
                Err(AegisError::CloudFallbackApprovalRequired { from: from_local_model.clone(), to: to_cloud_provider.clone() })
            }
            _ => Ok(PolicyDecision { risk_tier: RiskTier::Low, permitted: true, approval_request_id: None, reason: "Allowed".into() }),
        }
    }
}
