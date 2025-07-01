use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde::{Deserialize, Serialize};
use crate::prelude::TransferJobInternalDto;

impl From<TransferJob> for TransferJobInternalDto {
    fn from(value: TransferJob) -> Self {
        value.dto
    }
}

// const FAIL_MESSAGE: &'static str =
//     "Job cannot be recovered due to server data loss. Did it restart?";

impl From<TransferJobInternalDto> for TransferJob {
    fn from(value: TransferJobInternalDto) -> Self {
        TransferJob {
            dto: value,
            _thread: None,
            _cancel_rx: None,
            _cancel_tx: None,
        } // todo: add something for fail message here?
    }
}

impl AsRef<TransferJob> for TransferJob {
    fn as_ref(&self) -> &TransferJob {
        self
    }
}

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
    pub time: DateTime<FixedOffset>,
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
    pub dto: TransferJobInternalDto,
    pub _thread: Option<JoinHandle<()>>,
    pub _cancel_tx: Option<Sender<()>>,
    pub _cancel_rx: Option<Receiver<bool>>,
}

impl TransferJob {
    pub fn status(&self) -> TransferJobStatus {
        if self._thread.is_some() {
            TransferJobStatus::Running
        } else if self.dto.fatal_errors.len() > 0 {
            TransferJobStatus::Failed
        } else if self.dto.last_stats.is_none() {
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
