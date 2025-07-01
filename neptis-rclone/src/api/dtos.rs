use chrono::NaiveDateTime;
use neptis_lib::db::sync_models::{RCloneStat, TransferJob, TransferJobStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        let job_dto = &job_ref.dto;
        let stat = job_ref.status();
        TransferJobDto {
            job_id: job_dto.job_id.clone(),
            server_name: job_dto.server_name.clone(),
            smb_folder: job_dto.smb_folder.clone(),
            local_folder: job_dto.local_folder.clone(),
            errors: job_dto.fatal_errors.0.clone(),
            last_stats: job_dto.last_stats.clone().map(|x|x.0),
            start_date: job_dto.start_date.clone(),
            end_date: job_dto.end_date.clone(),
            last_updated: job_dto.last_updated.clone(),
            stat
        }
    }
}