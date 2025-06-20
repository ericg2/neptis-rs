use chrono::NaiveDateTime;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RCloneLogLevel {
    #[serde(alias = "notice")]
    #[serde(alias = "info")]
    Notice,
    
    #[serde(alias = "warn")]
    #[serde(alias = "warning")]
    Warning,
    
    #[serde(alias = "err")]
    #[serde(alias = "error")]
    #[serde(alias = "fatal")]
    Error,
    
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneMessage {
    pub level: RCloneLogLevel,
    pub msg: String,
    pub stats: RCloneStat,
    pub time: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneStat {
    pub bytes: u64,
    pub checks: u64,
    pub deleted_dirs: u64,
    pub deletes: u64,
    pub elapsed_time: f64,
    pub errors: u64,
    pub eta: Option<u64>,
    pub fatal_error: bool,
    pub listed: u64,
    pub renames: u64,
    pub retry_error: bool,
    pub server_side_copies: u64,
    pub server_side_copy_bytes: u64,
    pub server_side_move_bytes: u64,
    pub server_side_moves: u64,
    pub speed: u64,
    pub total_bytes: u64,
    pub total_checks: u64,
    pub total_transfers: u64,
    pub transfer_time: f64,
}
