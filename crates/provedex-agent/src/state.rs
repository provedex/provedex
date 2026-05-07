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

    /// Cheap probe: try to open the ledger file in append mode. Returns false
    /// if the file or its directory is read-only. Used by /v1/healthz so an
    /// operator monitoring the endpoint catches a silent breakage.
    pub fn ledger_writable(&self) -> bool {
        std::fs::OpenOptions::new()
            .append(true)
            .open(self.session.ledger().path())
            .is_ok()
    }
}
