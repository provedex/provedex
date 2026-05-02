#[derive(Debug, Clone, PartialEq)]
pub enum ChainStatus {
    Valid,
    Broken,
}

#[derive(Debug, Clone)]
pub struct ChainReport {
    pub status: ChainStatus,
    pub event_count: u64,
    pub broken_at_seq: Option<u64>,
    pub root_hash: String,
}

pub fn verify_chain<I>(_events: I) -> ChainReport
where
    I: IntoIterator,
{
    ChainReport {
        status: ChainStatus::Valid,
        event_count: 0,
        broken_at_seq: None,
        root_hash: String::new(),
    }
}
