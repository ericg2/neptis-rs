use crate::dtos::{RCloneJobDto, RCloneJobStatus};
use crate::errors::NeptisError;
use crate::models::RCloneStat;
use base64::prelude::*;
use chrono::{NaiveDateTime, Utc};
use duct::cmd;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::{Sender, channel};
use std::time::SystemTime;
use std::{
    fs,
    io::{BufRead, BufReader, Cursor, Read},
    ops::Deref,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread::JoinHandle,
};
use tokio::runtime::Runtime;
use url::Host::Ipv4;
use url::Url;
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

pub struct RCloneJob<'a> {
    settings: RCloneSettings,
    start_date: Option<NaiveDateTime>,
    end_date: Option<NaiveDateTime>,
    info: RCloneJobLaunchInfo<'a>,
    job_id: Uuid,
    batch_id: Uuid,
    stats: Arc<RwLock<RCloneStat>>,
    _thread: Option<(Sender<()>, JoinHandle<()>)>,
}

impl<'a> RCloneJob<'a> {
    fn get_status(&self, r_stat: &RCloneStat) -> RCloneJobStatus {
        if self.start_date.is_none() {
            RCloneJobStatus::NotStarted
        } else if r_stat.fatal_error {
            RCloneJobStatus::Failed
        } else if self.end_date.is_none() {
            RCloneJobStatus::Running
        } else {
            RCloneJobStatus::Successful
        }
    }
    pub fn stats(&self) -> RCloneJobDto {
        let r_stats = self.stats.read().unwrap();
        RCloneJobDto {
            batch_id: self.batch_id.clone(),
            job_id: self.job_id.clone(),
            error_count: r_stats.errors,
            status: self.get_status(r_stats.deref()),
            total_bytes: r_stats.total_bytes,
            done_bytes: r_stats.bytes,
            bytes_per_sec: r_stats.speed,
            working_files: r_stats
                .transferring
                .clone()
                .into_iter()
                .map(|x| x.into())
                .collect::<Vec<_>>(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
        }
    }
    pub fn stop(&mut self) -> Result<(), NeptisError> {
        if let Some((ref tx, _)) = self._thread {
            tx.send(())
                .map_err(|_| NeptisError::InternalError("Failed to send cancel signal!".into()))?;
            self._thread = None;
            Ok(())
        } else {
            Err(NeptisError::InternalError("Job is not running".into()))
        }
    }
    //noinspection RsFormatMacroWithoutFormatArguments
    pub fn start(&mut self) -> Result<(), NeptisError> {
        if self._thread.is_some() {
            return Err(NeptisError::InternalError("Job is already running!".into()));
        }
        let exe_path = self.settings.exe_path();
        let exe_path_str = exe_path.to_str().unwrap();

        let obs_pass = cmd!(exe_path_str, "obscure", &self.info.password).read()?;

        // First, make sure the config has a correct value.
        let host_id = BASE64_STANDARD.encode(&self.info.host).replace("=", "");
        let mut c_entry = String::new();
        c_entry += &format!("[{}]\n", &host_id);
        c_entry += &format!("type = smb\n");
        c_entry += &format!("host = {}\n", &self.info.host);
        c_entry += &format!("user = {}\n", &self.info.user_name);
        c_entry += &format!("pass = {}\n", &obs_pass);

        let config_path = self.settings.working_path.join(Uuid::new_v4().to_string());
        fs::write(&config_path, c_entry)?;

        let stats = Arc::new(RwLock::new(RCloneStat::default()));
        let stat_c = stats.clone();
        let cmd_exp = cmd!(
            exe_path_str,
            "sync",
            &self.info.local_folder,
            format!("{}:{}", host_id, self.info.remote_folder),
            "--use-json-log",
            "--stats",
            "1s",
            "--log-level",
            "NOTICE",
            "--stats-log-level",
            "NOTICE"
        )
        .env("RCLONE_CONFIG", config_path.to_str().unwrap())
        .stderr_to_stdout();

        // Attempt to run the command and view the output if not cancelled.
        let (tx, rx) = channel();
        self.start_date = Some(Utc::now().naive_utc());
        self._thread = Some((
            tx,
            std::thread::spawn(move || {
                match cmd_exp.reader() {
                    Ok(handle) => {
                        let rdr = BufReader::new(&handle);
                        for line in rdr.lines() {
                            if let Ok(line) = line {
                                // Attempt to parse with JSON to create a new statistic.
                                if let Ok(stat) = serde_json::from_str::<RCloneStat>(&line) {
                                    *stat_c.write().unwrap() = stat;
                                }
                            }
                            if rx.try_recv().is_ok() && handle.kill().is_ok() {
                                stat_c.write().unwrap().fatal_error = true;
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        stat_c.write().unwrap().fatal_error = true;
                        println!("{}", e);
                    }
                }
            }),
        ));
        Ok(())
    }
}

#[derive(Clone)]
pub struct RCloneSettings {
    working_path: PathBuf,
}

impl RCloneSettings {
    pub fn new<P: AsRef<Path>>(working_path: P) -> RCloneSettings {
        RCloneSettings {
            working_path: working_path.as_ref().to_path_buf(),
        }
    }
    pub fn exe_path(&self) -> PathBuf {
        self.working_path.join(FILE_NAME)
    }
}

pub struct RCloneClient<'a> {
    settings: RCloneSettings,
    all_jobs: Vec<RCloneJob<'a>>,
}

#[derive(Clone)]
pub struct RCloneJobLaunchInfo<'a> {
    pub host: &'a str,
    pub user_name: &'a str,
    pub password: &'a str,
    pub local_folder: &'a str,
    pub remote_folder: &'a str,
}

impl<'a> RCloneJobLaunchInfo<'a> {
    pub fn validate(&self) -> Result<Self, &'static str> {
        let mut item = self.clone();
        item.host = item.host.trim();
        item.user_name = item.user_name.trim();
        item.password = item.password.trim();
        item.local_folder = item.local_folder.trim();
        item.remote_folder = item.remote_folder.trim();

        if item.host.is_empty() {
            Err("The host cannot be empty!")
        } else if item.user_name.is_empty() {
            Err("The user name cannot be empty!")
        } else if item.password.is_empty() {
            Err("The password cannot be empty!")
        } else if item.local_folder.is_empty() {
            Err("The local folder cannot be empty!")
        } else if item.remote_folder.is_empty() {
            Err("The remote folder cannot be empty!")
        } else if Url::parse(&item.host).is_err() && IpAddr::from_str(&item.host).is_err() {
            Err("The host must be a valid URL!")
        } else if !PathBuf::from_str(&item.local_folder).is_ok_and(|x| x.exists()) {
            Err("The local folder must be an existing, valid location!")
        } else {
            Ok(item)
        }
    }
}

impl<'a> RCloneClient<'a> {
    fn _ensure_check(&mut self) -> Result<(), NeptisError> {
        // First, make sure the parent directories exist before continuing.
        let exe_path = self.settings.exe_path();

        fs::create_dir_all(&self.settings.working_path)?;
        let min_date: SystemTime = Utc::now().into();
        if fs::metadata(&exe_path)
            .ok()
            .map(|x| x.accessed().ok())
            .flatten()
            .is_some_and(|x| x < min_date)
        {
            Ok(())
        } else {
            let rt = Runtime::new()?;
            let b_zip = rt.block_on(async move {
                Ok::<Vec<u8>, NeptisError>(
                    reqwest::get(DOWNLOAD_URL)
                        .await?
                        .error_for_status()?
                        .bytes()
                        .await?
                        .to_vec(),
                )
            })?;
            let cursor = Cursor::new(b_zip);
            let mut archive = ZipArchive::new(cursor)?;
            let file_bytes = (0..archive.len())
                .map(|i| {
                    let mut file = archive.by_index(i).ok()?;
                    if file.name().ends_with(FILE_NAME) {
                        let mut buffer = vec![];
                        file.read_to_end(&mut buffer).ok()?;
                        return Some(buffer);
                    }
                    None
                })
                .find_map(|x| x)
                .ok_or(NeptisError::InternalError(
                    "Failed to find file in ZIP archive!".into(),
                ))?;
            fs::write(&exe_path, file_bytes)?;
            Ok(())
        }
    }

    /// Returns: The UUID of the created batch.
    pub fn create_batch(&mut self, infos: &[RCloneJobLaunchInfo<'a>]) -> Result<Uuid, NeptisError> {
        self._ensure_check()?;
        let b_id = Uuid::new_v4();
        for info in infos {
            self.all_jobs.push(RCloneJob {
                settings: self.settings.clone(),
                start_date: None,
                end_date: None,
                info: info
                    .validate()
                    .map_err(|x| NeptisError::BadRequest(x.into()))?,
                job_id: Uuid::new_v4(),
                batch_id: b_id.clone(),
                stats: Arc::new(RwLock::new(RCloneStat::default())),
                _thread: None,
            })
        }
        Ok(b_id.clone())
    }

    pub fn create_job(
        &'a mut self,
        info: RCloneJobLaunchInfo<'a>,
    ) -> Result<&'a mut RCloneJob<'a>, NeptisError> {
        let b_id = { self.create_batch(&[info])? }.clone();
        Ok(self
            .all_jobs
            .iter_mut()
            .find(|x| x.batch_id == b_id)
            .unwrap())
    }

    pub fn all_jobs(&'a mut self) -> &'a mut [RCloneJob<'a>] {
        &mut self.all_jobs
    }

    pub fn new(settings: RCloneSettings) -> RCloneClient<'a> {
        RCloneClient {
            settings,
            all_jobs: vec![],
        }
    }
}
