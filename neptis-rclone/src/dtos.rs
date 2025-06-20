use crate::models::RCloneFileTransferStat;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::alloc::GlobalAlloc;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub enum RCloneJobStatus {
    NotStarted,
    Running,
    Successful,
    Failed
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RCloneJobDto {
    pub batch_id: Uuid,
    pub job_id: Uuid,
    pub error_count: u64,
    pub status: RCloneJobStatus,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub bytes_per_sec: f64,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
}