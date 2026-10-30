//! Interactive Terminal Dashboard & Chat ViewModel (ratatui).
use aegis_core::{AgentActivityState, AutonomousTask, CommandApprovalRequest};
use aegis_events::LiveActivityEvent;

#[derive(Debug, Clone)]
pub struct TerminalDashboardViewModel {
    pub active_task: Option<AutonomousTask>,
    pub recent_events: Vec<LiveActivityEvent>,
    pub pending_approval: Option<CommandApprovalRequest>,
    pub status_banner: String,
}

impl TerminalDashboardViewModel {
    pub fn render_ascii_snapshot(&self) -> String {
        let state_str = self.active_task.as_ref().map(|t| format!("{:?}", t.state)).unwrap_or_else(|| format!("{:?}", AgentActivityState::Initializing));
        format!("+--- AEGIS TERMINAL TUI ({}) | State: {} ---+\n", self.status_banner, state_str)
    }
}
