use chrono::NaiveDateTime;
use sqlx::FromRow;
use sqlx::types::Json;
use uuid::Uuid;
use crate::db::sync_models::RCloneStat;

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoSchedule {
    pub schedule_name: String,
    pub server_name: String,
    pub cron_schedule: String,
    pub smb_user_name: String,
    pub smb_password: String,
    pub user_password: Option<String>,
    pub backup_on_finish: bool,
    pub last_updated: NaiveDateTime
}

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoJob {
    pub schedule_name: String,
    pub server_name: String,
    pub action_name: String,
    pub smb_folder: String, // this will be converted to a point name
    pub local_folder: String,
    pub enabled: bool,
}

#[derive(Clone, FromRow, PartialEq)]
pub struct TransferJobInternalDto {
    pub job_id: Uuid,
    pub auto_job_action_name: Option<String>,
    pub auto_job_schedule_name: Option<String>,
    pub server_name: String,
    pub smb_user_name: String,
    pub smb_password: String,
    pub smb_folder: String,
    pub local_folder: String,
    pub last_stats: Option<Json<RCloneStat>>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub fatal_errors: Json<Vec<String>>,
    pub warnings: Json<Vec<String>>,
    pub last_updated: NaiveDateTime,
    pub init_hash: Option<String>,
}

