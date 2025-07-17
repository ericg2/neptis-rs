use crate::ipc::errors::ApiError;
use crate::prelude::{DbController, WebApi};
use crate::rolling_secret::RollingSecret;
use chrono::{NaiveDateTime, Utc};
use notify_rust::Notification;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

struct IPCSession {
    api: WebApi,
    errors: u64,
}

impl IPCSession {
    pub fn new(api: WebApi) -> Self {
        Self { api, errors: 0 }
    }
}

struct IPCSessionBlacklist {
    endpoint: String,
    expire_date: NaiveDateTime,
}

impl IPCSessionBlacklist {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            expire_date: Utc::now().naive_utc() + Duration::from_secs(300),
        }
    }
}

pub struct IPCMessageReceiver {
    sessions: Vec<IPCSession>,
    db: Arc<DbController>,
    rt: Arc<Runtime>,
    blacklists: Vec<IPCSessionBlacklist>,
}

const MAX_ERRORS: u64 = 5;

impl IPCMessageReceiver {
    pub fn new(db: Arc<DbController>, rt: Arc<Runtime>) -> Self {
        Self {
            sessions: vec![],
            blacklists: vec![],
            db,
            rt,
        }
    }

    pub fn _thread_iter(&mut self) -> Result<(), ApiError> {
        // First, attempt to pull all sessions from the database.
        self.blacklists
            .retain(|x| x.expire_date < Utc::now().naive_utc());

        for (endpoint, user_name, user_pass, key) in self
            .rt
            .block_on(async { self.db.get_all_servers().await })?
            .into_iter()
            .filter_map(|x| {
                if let Some(user_name) = x.user_name
                    && let Some(user_pass) = x.user_password
                    && !self
                        .blacklists
                        .iter()
                        .any(|y| y.endpoint == x.server_endpoint)
                {
                    Some((
                        x.server_endpoint,
                        user_name,
                        user_pass,
                        x.server_password
                            .and_then(|x| RollingSecret::from_string(&x)),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
        {
            // If no session exists, attempt to create it. Delete any old entries with the
            // same matching endpoint - as this means the username and/or password changed.
            if !self.sessions.iter().any(|x| {
                x.api.get_endpoint() == endpoint
                    && x.api.get_username() == user_name
                    && x.api.get_password() == user_pass
            }) {
                // 7-9-25: DO NOT TRIGGER A WAKEUP HERE!
                let api = WebApi::new(&endpoint, &user_name, &user_pass, key);
                if self.rt.block_on(async { api.get_info().await }).is_ok() {
                    self.sessions.retain(|x| x.api.get_endpoint() != endpoint);
                    self.sessions.push(IPCSession::new(api));
                }
                else {
                    // Put the server on a temporary blacklist to try again later. This
                    // avoids putting excessive strain on the computer pulling the data.
                    self.blacklists.push(IPCSessionBlacklist::new(endpoint));
                }
            }
        }

        // Do not keep pinging the same server if it disconnects!
        self.sessions.retain(|x| x.errors <= MAX_ERRORS);

        // Next, try to pull all new messages and send a notification.
        for session in self.sessions.iter_mut() {
            if let Ok(new_messages) = self
                .rt
                .block_on(async { session.api.get_all_messages(true).await })
            {
                for notification in new_messages {
                    println!("New message received!");
                    let _ = Notification::new()
                        .summary(&notification.subject.unwrap_or("New Message".into()))
                        .body(&notification.message)
                        .appname("Neptis")
                        .show();
                }
            } else {
                session.errors += 1;
            }
        }
        Ok(())
    }

    pub fn handle_blocking(&mut self) {
        loop {
            if let Err(e) = self._thread_iter() {
                println!("Failed to run notify: {}", e);
            }
            thread::sleep(Duration::from_secs(15));
        }
    }
}
