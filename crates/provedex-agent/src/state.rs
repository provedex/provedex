use provedex_core::LedgerSession;

/// Owns the `LedgerSession` the agent serves over HTTP. One session per agent
/// process. Multi-tenant isolation lands in a follow-up (see ADR 0004).
pub struct AgentState {
    pub session: LedgerSession,
}

impl AgentState {
    pub fn new(session: LedgerSession) -> Self {
        Self { session }
    }
}
