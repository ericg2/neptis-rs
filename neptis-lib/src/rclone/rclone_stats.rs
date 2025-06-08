use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RCloneLogLevel {
    Debug,
    Info,
    Warning,
    Error,
    #[serde(other)] // fallback if unexpected string (like "fatal")
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneMessage {
    pub level: RCloneLogLevel,

    #[serde(rename = "msg")]
    pub message: String,
    pub stats: RCloneStat,
    pub time: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneStat {
    #[serde(rename = "bytes")]
    pub processed_bytes: i64,

    #[serde(rename = "elapsed_time")]
    pub elapsed_secs: f64,

    pub errors: i64,
    pub fatal_error: bool,
    pub retry_error: bool,
    pub server_side_copies: i64,
    pub server_side_copy_bytes: i64,
    pub server_side_move_bytes: i64,
    pub server_side_moves: i64,

    #[serde(rename = "speed")]
    pub speed_bytes: f64,

    pub total_bytes: i64,
    pub total_checks: i64,
    pub total_transfers: i64,

    #[serde(rename = "transfer_time")]
    pub transfer_secs: f64,

    pub transferring: Vec<RCloneFileTransferStat>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneFileTransferStat {
    pub bytes: i64,
    pub dst_fs: String,
    pub group: String,
    pub name: String,
    pub percentage: i64,
    pub size: i64,
    pub speed: f64,
    pub speed_avg: i64,
    pub src_fs: String,
}
