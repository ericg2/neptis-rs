use sqlx::{FromRow, Row};

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoSchedule {
    pub schedule_name: String,
    pub server_name: String,
    pub cron_schedule: String,
    pub smb_user_name: String,
    pub smb_password: String,
}

#[derive(Clone, FromRow, Eq, PartialEq)]
pub struct TransferAutoJob {
    pub schedule_name: String,
    pub server_name: String,
    pub action_name: String,
    pub smb_folder: String, // FIXME: make sure to convert from SMB!
    pub local_folder: String,
}

