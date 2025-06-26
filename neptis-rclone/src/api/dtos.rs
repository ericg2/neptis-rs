use crate::models::{TransferJobStatus, RCloneStat, TransferJob};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use neptis_lib::prelude::ServerItem;

#[derive(Clone, Serialize, Deserialize)]
pub struct TransferJobDto {
    pub job_id: Uuid,
    pub server_name: String,
    pub smb_folder: String,
    pub local_folder: String,
    pub stat: TransferJobStatus,
    pub errors: Vec<String>,
    pub last_stats: Option<RCloneStat>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub last_updated: NaiveDateTime,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PostForAutoScheduleStartDto {
    pub server_name: String,
    pub schedule_name: String,
}

impl <T: AsRef<TransferJob>> From<T> for TransferJobDto {
    fn from(job: T) -> Self {
        let job_ref = job.as_ref();
        let stat = job_ref.status();
        TransferJobDto {
            job_id: job_ref.job_id.clone(),
            server_name: job_ref.server.server_name.clone(),
            smb_folder: job_ref.smb_folder.clone(),
            local_folder: job_ref.local_folder.clone(),
            errors: job_ref.fatal_errors.clone(),
            last_stats: job_ref.last_stats.clone(),
            start_date: job_ref.start_date.clone(),
            end_date: job_ref.end_date.clone(),
            last_updated: job_ref.last_updated.clone(),
            stat
        }
    }
}

impl AsRef<TransferJob> for TransferJob {
    fn as_ref(&self) -> &TransferJob {
        self
    }
}