use crate::dtos::TransferJobDto;
use crate::errors::NeptisError;
use crate::models::{RCloneStat, TransferJob, TransferJobStatus};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use duct::cmd;
use neptis_lib::prelude::DbController;
use rocket::yansi::Paint;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use std::{
    fs,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
    thread,
};
use tokio::runtime::Runtime;
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

pub struct RCloneClient {
    settings: RCloneSettings,
    jobs: Arc<Mutex<Vec<TransferJob>>>,
    db: Arc<DbController>,
}

#[derive(Clone)]
pub struct RCloneJobLaunchInfo {
    pub server_name: String,
    pub smb_user_name: String,
    pub smb_password: String,
    pub local_folder: String,
    pub smb_folder: String,
}

impl RCloneJobLaunchInfo {
    pub fn validate(&self) -> Result<Self, &'static str> {
        let mut item = self.clone();
        item.server_name = item.server_name.trim().to_string();
        item.smb_user_name = item.smb_user_name.trim().to_string();
        item.smb_password = item.smb_password.trim().to_string();
        item.local_folder = item.local_folder.trim().to_string();
        item.smb_folder = item.smb_folder.trim().to_string();

        if item.server_name.is_empty() {
            Err("The host cannot be empty!")
        } else if item.smb_user_name.is_empty() {
            Err("The user name cannot be empty!")
        } else if item.smb_password.is_empty() {
            Err("The password cannot be empty!")
        } else if item.local_folder.is_empty() {
            Err("The local folder cannot be empty!")
        } else if item.smb_folder.is_empty() {
            Err("The remote folder cannot be empty!")
        } else if Url::parse(&item.server_name).is_err()
            && IpAddr::from_str(&item.server_name).is_err()
        {
            Err("The host must be a valid URL!")
        } else if !PathBuf::from_str(&item.local_folder).is_ok_and(|x| x.exists()) {
            Err("The local folder must be an existing, valid location!")
        } else {
            Ok(item)
        }
    }
    pub fn new<S: Into<String>>(
        host: S,
        user_name: S,
        password: S,
        local_folder: S,
        remote_folder: S,
    ) -> Self {
        RCloneJobLaunchInfo {
            server_name: host.into(),
            smb_user_name: user_name.into(),
            smb_password: password.into(),
            local_folder: local_folder.into(),
            smb_folder: remote_folder.into(),
        }
    }
}

impl RCloneClient {
    fn _find_smb_address(url_str: impl AsRef<str>) -> Result<String, NeptisError> {
        Url::parse(url_str.as_ref())
            .map(|x| x.host_str().map(|y| y.to_string()))
            .or_else(|_| {
                url_str
                    .as_ref()
                    .parse::<IpAddr>()
                    .map(|x| Some(x.to_string()))
            })
            .ok()
            .flatten()
            .ok_or(NeptisError::BadRequest("Not a valid URL or IP".into()))
    }
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
    //noinspection RsFormatMacroWithoutFormatArguments
    fn _start_job(&mut self, job_id: Uuid) -> Result<(), NeptisError> {
        self._ensure_check()?; // **** make sure we are okay!
        let _lock = &mut *self.jobs.lock().unwrap();
        let job = _lock
            .iter_mut()
            .filter(|x| x.status() != TransferJobStatus::Running)
            .find(|x| x.job_id == job_id)
            .ok_or(NeptisError::BadRequest(
                "Job ID does not exist or is running already!".into(),
            ))?;

        let exe_path = self.settings.exe_path();
        let exe_path_str = exe_path.to_str().unwrap();

        let host_id = BASE64_STANDARD
            .encode(&job.server.server_name)
            .replace("=", "");

        let mut c_entry = String::new();
        {
            let host = Self::_find_smb_address(&job.server.server_endpoint)?;
            let pass = cmd!(exe_path_str, "obscure", &job.smb_password).read()?;
            c_entry += &format!("[{}]\n", &host_id);
            c_entry += &format!("type = smb\n");
            c_entry += &format!("host = {}\n", host);
            c_entry += &format!("user = {}\n", &job.smb_user_name);
            c_entry += &format!("pass = {}\n", pass);
        }

        let config_path = self.settings.working_path.join(Uuid::new_v4().to_string());
        fs::write(&config_path, c_entry)?;

        let cmd_exp = cmd!(
            exe_path_str,
            "sync",
            &job.local_folder,
            format!("{}:{}", host_id, &job.smb_folder),
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

        // Set the start date and attempt to pass it off to the thread.
        let jobs = self.jobs.clone();
        let (s_tx, s_rx) = channel::<()>();
        let (r_tx, r_rx) = channel::<bool>();
        job.start_date = Some(Utc::now().naive_utc());
        job.fatal_errors = vec![];
        job.warnings = vec![];
        job.last_stats = None;
        job._cancel_tx = Some(s_tx);
        job._cancel_rx = Some(r_rx);
        job._thread = Some(thread::spawn(move || {
            Self::_handle_job(job_id, cmd_exp, jobs, s_rx, r_tx)
        }));
        Ok(())
    }

    fn _handle_job(
        job_id: Uuid,
        cmd: duct::Expression,
        jobs: Arc<Mutex<Vec<TransferJob>>>,
        s_rx: Receiver<()>,
        r_tx: Sender<bool>,
    ) {
        let mark_message = |msg: &str, fatal: bool, stat: Option<RCloneStat>| {
            let _lock = &mut *jobs.lock().unwrap();
            let current_now = Utc::now().naive_utc();
            let job = _lock
                .iter_mut()
                .find(|x| x.job_id == job_id)
                .expect("Expected job to exist after creation!");
            if !msg.is_empty() {
                if fatal {
                    job.fatal_errors.push(msg.into());
                } else {
                    job.warnings.push(msg.into());
                }
            }
            if let Some(stat) = stat {
                job.last_stats = Some(stat); // *** prevent a None value from overwriting.
            }
            if fatal {
                // We can assume the thread has already been disconnected here.
                job.end_date = Some(current_now);
                job._thread = None;
                job._cancel_tx = None;
                job._cancel_rx = None;
            }
            job.last_updated = current_now;
        };
        match cmd.reader() {
            Ok(handle) => {
                let rdr = BufReader::new(&handle);
                for line in rdr.lines() {
                    match line {
                        Ok(line) => {
                            if let Ok(stat) = serde_json::from_str::<RCloneStat>(&line) {
                                mark_message("", false, Some(stat));
                            }
                        }
                        Err(e) => {
                            let msg = &format!("Failed to read line! Error: {e}");
                            mark_message(msg, false, None);
                        }
                    }
                    // If we are receiving a KILL request - process it here!
                    if s_rx.try_recv().is_ok() {
                        if handle.kill().is_ok() {
                            let _ = r_tx.send(true);
                            mark_message("Operation cancelled", true, None);
                            return;
                        } else {
                            let _ = r_tx.send(false);
                            mark_message("Operation failed to cancel", false, None);
                        }
                    }
                }
                mark_message("", true, None); // *** job has finished!
                return;
            }
            Err(e) => {
                let err = &format!("Failed to pull reader! Error: {e}");
                mark_message(err, true, None)
            }
        }
    }

    ////////////////////////////////////////////////// all public methods below

    pub fn cancel_job(&mut self, job_id: Uuid) -> Result<(), NeptisError> {
        let _lock = &mut *self.jobs.lock().unwrap();
        let job = _lock
            .iter_mut()
            .filter(|x| x.status() == TransferJobStatus::Running)
            .find(|x| x.job_id == job_id)
            .ok_or(NeptisError::BadRequest(
                "Job ID does not exist or is not running!".into(),
            ))?;
        let tx = job._cancel_tx.as_ref().ok_or(NeptisError::InternalError(
            "Cancel is not supported (no TX)".into(),
        ))?;
        let rx = job._cancel_rx.as_ref().ok_or(NeptisError::InternalError(
            "Cancel is not supported (no RX)".into(),
        ))?;
        tx.send(())
            .map_err(|_| NeptisError::InternalError("Failed to send request!".into()))?;
        rx.recv_timeout(Duration::from_secs(3))
            .ok()
            .filter(|&x| x)
            .map(|_| ())
            .ok_or(NeptisError::InternalError(
                "Timeout exceeded or failed to cancel!".into(),
            ))
    }

    /// Returns: The UUID of the created batch.
    pub fn create_batch(&self, infos: Vec<RCloneJobLaunchInfo>) -> Result<Uuid, NeptisError> {
        let b_id = Uuid::new_v4();
        let all_servers = self.db.get_all_servers_sync().map_err(|x| {
            NeptisError::InternalError(format!("Failed to pull all servers: {}", x.to_string()))
        })?;
        for info in infos {
            // Attempt to pull the Server Name from the DB Controller first.
            let server = all_servers
                .iter()
                .find(|x| x.server_name == info.server_name)
                .map(|x| x.to_owned())
                .ok_or(NeptisError::BadRequest("Cannot locate server!".into()))?;
            {
                let y = &mut *self.jobs.lock().unwrap();
                y.push(TransferJob {
                    job_id: Uuid::new_v4(),
                    batch_id: b_id.clone(),
                    server,
                    smb_user_name: info.smb_user_name,
                    smb_password: info.smb_password,
                    smb_folder: info.smb_folder,
                    local_folder: info.local_folder,
                    last_stats: None,
                    start_date: None,
                    end_date: None,
                    fatal_errors: vec![],
                    warnings: vec![],
                    last_updated: Utc::now().naive_utc(),
                    _thread: None,
                    _cancel_tx: None,
                    _cancel_rx: None,
                })
            }
        }
        Ok(b_id.clone())
    }

    pub fn get_job(&self, job_id: Uuid) -> Option<TransferJobDto> {
        let _lock = &*self.jobs.lock().unwrap();
        _lock
            .iter()
            .find(|x| x.job_id == job_id)
            .map(|x| x.into())
    }

    pub fn get_batch(&self, batch_id: Uuid) -> Vec<TransferJobDto> {
        let _lock = &*self.jobs.lock().unwrap();
        _lock
            .iter()
            .filter(|x| x.batch_id == batch_id)
            .map(|x| x.into())
            .collect::<Vec<_>>()
    }

    pub fn start_batch(&mut self, batch_id: Uuid) -> Result<(), NeptisError> {
        let all_jobs = { self.get_batch(batch_id) };
        for job in all_jobs {
            self._start_job(job.job_id)?
        }
        Ok(())
    }

    pub fn cancel_batch(&mut self, batch_id: Uuid) -> Result<(), NeptisError> {
        let all_jobs = { self.get_batch(batch_id) };
        for job in all_jobs {
            self.cancel_job(job.job_id)?
        }
        Ok(())
    }

    pub fn new(
        settings: RCloneSettings,
        jobs: Arc<Mutex<Vec<TransferJob>>>,
        db: Arc<DbController>,
    ) -> Self {
        Self { settings, jobs, db }
    }

    pub fn new_owned(settings: RCloneSettings, db: Arc<DbController>) -> Self {
        Self::new(settings, Arc::new(Mutex::new(vec![])), db)
    }
}
