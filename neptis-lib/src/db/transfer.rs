use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Default, Serialize, Deserialize, FromRow)]
pub struct AutoTransfer {
    pub id: Uuid,
    pub server_name: String,
    pub user_name: String,
    pub user_password: String,
    pub point_name: String,
    pub cron_schedule: String,
    pub last_ran: Option<NaiveDateTime>,
}
