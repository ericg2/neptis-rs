use crate::traits::ToShortIdString;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, Clone, Default, FromRow)]
pub struct ServerItem {
    pub server_name: String,
    pub server_endpoint: String,
    pub server_password: Option<String>,
    pub user_name: Option<String>,
    pub user_password: Option<String>,
    pub arduino_endpoint: Option<String>,
    pub arduino_password: Option<String>,
    pub auto_fuse: bool,
    pub is_default: bool,
}

impl ToShortIdString for ServerItem {
    fn to_short_id_string(&self) -> String {
        self.server_name.clone()
    }
}
