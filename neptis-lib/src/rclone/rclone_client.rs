use std::{
    fs, io::{BufRead, BufReader, Cursor, Read}, ops::Deref, path::PathBuf, process::Command, sync::{Arc, RwLock}, thread::JoinHandle, time::Duration
};

use crate::{apis::NeptisError, db::transfer::AutoTransfer, rclone::rclone_stats::RCloneStat};
use base64::prelude::*;
use chrono::{NaiveDateTime, Utc};
use dirs_next::home_dir;
use duct::cmd;
use reqwest::Client;
use tokio::runtime::Runtime;
use uuid::Uuid;
use zip::ZipArchive;

#[cfg(target_os = "windows")]
const DOWNLOAD_URL: &'static str = "https://downloads.rclone.org/rclone-current-windows-amd64.zip";

#[cfg(target_os = "linux")]
const DOWNLOAD_URL: &'static str = "https://downloads.rclone.org/rclone-current-linux-amd64.zip";

#[cfg(target_os = "macos")]
const DOWNLOAD_URL: &'static str = "https://downloads.rclone.org/rclone-current-osx-amd64.zip";

#[cfg(target_os = "windows")]
const FILE_NAME: &'static str = "rclone.exe";

#[cfg(not(target_os = "windows"))]
const FILE_NAME: &'static str = "rclone";

pub struct RCloneJob {
    pub start_date: NaiveDateTime,
    stats: Arc<RwLock<RCloneStat>>,
    _thread: JoinHandle<()>,
}

impl RCloneJob {
    pub fn stats(&self) -> RCloneStat {
        self.stats.read().unwrap().clone()
    }
}

pub struct RCloneClient {
    exe_path: String,
    running_jobs: Vec<RCloneJob>,
}

pub struct RCloneJobLaunchInfo {
    pub host: String,
    pub user_name: String,
    pub password: String,
    pub local_folder: String,
    pub remote_folder: String,
}

impl RCloneClient {
    pub fn start_smb(&mut self, info: &RCloneJobLaunchInfo) -> Result<&RCloneJob, NeptisError> {
        let obs_pass = cmd!(&self.exe_path, "obscure", &info.password).read()?;

        // First, make sure the config has a correct value.
        let host_id = BASE64_STANDARD.encode(&info.host);
        let mut c_entry = String::new();
        c_entry += &format!("[{}]\n", &host_id);
        c_entry += &format!("type = smb\n");
        c_entry += &format!("host = {}\n", &info.host);
        c_entry += &format!("pass = {}\n", &obs_pass);
        c_entry += &format!("user = {}", &info.user_name);

        let config_path = home_dir()
            .ok_or(NeptisError::Str("Failed to find home directory!".into()))?
            .join(".neptis")
            .join(Uuid::new_v4().to_string());
        fs::write(&config_path, c_entry)?;

        let stats = Arc::new(RwLock::new(RCloneStat::default()));
        let stat_c = stats.clone();
        let cmd_exp = cmd!(
            &self.exe_path,
            "sync",
            &info.local_folder,
            format!("{}:{}", &host_id, &info.remote_folder),
            format!("--config {}", &config_path.to_str().unwrap()),
            "--use-json-log",
            "--stats 1s",
            "--log-level NOTICE",
            "--stats-log-level NOTICE"
        )
        .stderr_to_stdout();

        let job = RCloneJob {
            stats,
            start_date: Utc::now().naive_utc(),
            _thread: std::thread::spawn(move || {
                if let Ok(handle) = cmd_exp.reader() {
                    let rdr = BufReader::new(&handle);
                    for line in rdr.lines() {
                        if let Ok(line) = line {
                            // Attempt to parse with JSON to create a new statistic.
                            if let Ok(stat) = serde_json::from_str::<RCloneStat>(&line) {
                                *stat_c.write().unwrap() = stat;
                            }
                        }
                    }
                } else {
                    stat_c.write().unwrap().fatal_error = true;
                }
            }),
        };
        self.running_jobs.push(job);
        Ok(self.running_jobs.last().expect("Expected job to populate in vector!"))
    }
    pub fn new(rt: Arc<Runtime>) -> Result<RCloneClient, NeptisError> {
        let target_path = home_dir()
            .map(|x| x.join(".neptis").join(FILE_NAME))
            .ok_or(NeptisError::Str("Failed to find home directory!".into()))?;

        rt.block_on(async move {
            // If the file is newer than a week, let it go.
            if async {
                if tokio::fs::try_exists(&target_path).await.ok()?
                    && tokio::fs::metadata(&target_path)
                        .await
                        .ok()?
                        .created()
                        .ok()?
                        .elapsed()
                        .ok()?
                        < Duration::from_secs(604800)
                {
                    Some(())
                } else {
                    None
                }
            }
            .await
            .is_none()
            {
                let b_zip = reqwest::get(DOWNLOAD_URL)
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?
                    .to_vec();
                let cursor = Cursor::new(b_zip);
                let mut archive = ZipArchive::new(cursor)?;
                let new_bytes = {
                    let mut ret = Err(NeptisError::Str("Failed to find file in ZIP!".into()));
                    for i in 0..archive.len() {
                        let mut file = archive.by_index(i)?;
                        if file.name().ends_with(FILE_NAME) {
                            let mut buffer = vec![];
                            file.read_to_end(&mut buffer)?;
                            ret = Ok(buffer);
                            break;
                        }
                    }
                    ret
                }?;
                tokio::fs::write(&target_path, new_bytes).await?;
            }
            Ok(RCloneClient {
                exe_path: target_path.to_str().unwrap().to_string(),
                running_jobs: vec![]
            })
        })
    }
}
