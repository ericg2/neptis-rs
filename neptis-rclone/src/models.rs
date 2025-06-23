use chrono::NaiveDateTime;
use neptis_lib::prelude::ServerItem;
use rocket::serde::Serialize;
use serde::Deserialize;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneMessage {
    pub level: RCloneLogLevel,
    pub msg: String,
    pub stats: RCloneStat,
    pub time: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneStat {
    pub bytes: u64,
    pub speed: u64,
    pub checks: u64,
    pub deletes: u64,
    pub listed: u64,
    pub renames: u64,
    pub retry_error: bool,
    pub deleted_dirs: u64,
    pub server_side_copies: u64,
    pub server_side_copy_bytes: u64,
    pub server_side_move_bytes: u64,
    pub server_side_moves: u64,
    pub total_bytes: u64,
    pub total_checks: u64,
    pub total_transfers: u64,
}

pub struct TransferJob {
    pub job_id: Uuid,
    pub batch_id: Uuid,
    pub server: ServerItem,
    pub smb_user_name: String,
    pub smb_password: String,
    pub smb_folder: String,
    pub local_folder: String,
    pub last_stats: Option<RCloneStat>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub fatal_errors: Vec<String>,
    pub warnings: Vec<String>,
    pub last_updated: NaiveDateTime,
    pub _thread: Option<JoinHandle<()>>,
    pub _cancel_tx: Option<Sender<()>>,
    pub _cancel_rx: Option<Receiver<bool>>,
}

impl TransferJob {
    pub fn status(&self) -> TransferJobStatus {
        if self._thread.is_some() {
            TransferJobStatus::Running
        } else if self.fatal_errors.len() > 0 {
            TransferJobStatus::Failed
        } else if self.last_stats.is_none() {
            TransferJobStatus::NotStarted
        } else {
            TransferJobStatus::Successful
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub enum TransferJobStatus {
    NotStarted,
    Running,
    Successful,
    Failed,
}
