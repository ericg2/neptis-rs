use crate::apis::NeptisError;
use serde::{Deserialize, Serialize};
use std::fs;

use super::manager::ToShortIdString;

const SERVER_PATH: &'static str = "servers.json";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ServerItem {
    pub server_name: String,
    pub server_endpoint: String,
    pub server_password: Option<String>,
    pub user_name: Option<String>,
    pub arduino_endpoint: Option<String>,
    pub arduino_password: Option<String>,
}

impl ToShortIdString for ServerItem {
    fn to_short_id_string(&self) -> String {
        self.server_name.clone()
    }
}

impl ServerItem {
    pub fn load_servers() -> Result<Vec<ServerItem>, NeptisError> {
        if !fs::exists(SERVER_PATH)? {
            fs::write(SERVER_PATH, "[]")?;
        }
        let b = fs::read(SERVER_PATH)?;
        Ok(serde_json::from_slice(b.as_slice())?)
    }

    pub fn save_servers(items: &[ServerItem]) -> Result<(), NeptisError> {
        Ok(fs::write(SERVER_PATH, serde_json::to_string(items)?)?)
    }

    pub fn add_server(item: &ServerItem) -> Result<(), NeptisError> {
        let mut servers = Self::load_servers()?;
        servers.push(item.clone());
        Self::save_servers(servers.as_slice())
    }

    pub fn delete_server(name: &str) -> Result<(), NeptisError> {
        Self::save_servers(
            Self::load_servers()?
                .into_iter()
                .filter(|x| x.server_name != name)
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}
