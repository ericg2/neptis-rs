use cron::Schedule;
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoSchedule {
    pub schedule_name: String,
    pub server_name: String,
    pub cron_schedule: String,
}

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoJob {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub schedule_name: String,
    pub smb_user_name: String,
    pub smb_password: String,
    pub smb_folder: String,
    pub local_folder: String,
}