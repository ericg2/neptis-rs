use crate::api::dtos::{PostForAutoScheduleStartDto, TransferJobDto};
use crate::errors::ApiError;
use crate::models::{RCloneStat, TransferJob, TransferJobStatus};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::Utc;
use cron::{Schedule, TimeUnitSpec};
use duct::cmd;
use neptis_lib::prelude::DbController;
use rocket::futures::task::Spawn;
use rocket::yansi::Paint;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
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

#[derive(Clone)]
pub struct RCloneJobLaunchInfo {
    pub server_name: String,
    pub smb_user_name: String,
    pub smb_password: String,
    pub local_folder: String,
    pub smb_folder: String,
    pub auto_job: Option<String>,
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
            auto_job: None,
        }
    }
}

pub struct RCloneClient {
    settings: RCloneSettings,
    jobs: Arc<Mutex<Vec<TransferJob>>>,
    db: Arc<DbController>,
    start_tx: Mutex<Option<Sender<PostForAutoScheduleStartDto>>>,
}

impl RCloneClient {
    fn _find_smb_address(url_str: impl AsRef<str>) -> Result<String, ApiError> {
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
            .ok_or(ApiError::BadRequest("Not a valid URL or IP".into()))
    }
    fn _ensure_check(&self) -> Result<(), ApiError> {
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
                Ok::<Vec<u8>, ApiError>(
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
                .ok_or(ApiError::InternalError(
                    "Failed to find file in ZIP archive!".into(),
                ))?;
            fs::write(&exe_path, file_bytes)?;
            Ok(())
        }
    }

    fn _thread_iter(
        &self,
        start_rx: &Receiver<PostForAutoScheduleStartDto>,
    ) -> Result<(), ApiError> {
        let mut start_jobs = vec![];
        loop {
            match start_rx.try_recv() {
                Ok(x) => start_jobs.push(x),
                Err(_) => break,
            }
        }
        let all_jobs = &mut *self.jobs.lock().unwrap();
        let all_schedules = self.db.get_all_transfer_auto_schedules_sync()?;
        for job in self.db.get_all_transfer_auto_jobs_sync()? {
            // Find the schedule and see if a job is currently running or not.
            if let Some((job_schedule, cron_schedule)) = all_schedules
                .iter()
                .find(|x| x.schedule_name == job.schedule_name && x.server_name == job.server_name)
                .map(|x| Schedule::from_str(&x.cron_schedule).ok().map(|y| (x, y)))
                .flatten()
            {
                let related_jobs = all_jobs
                    .iter()
                    .filter(|x| {
                        x.server.server_name == job.server_name
                            && x.smb_folder == job.smb_folder
                            && x.local_folder == job.local_folder
                            && x.auto_job == Some(job.action_name.clone())
                    })
                    .collect::<Vec<_>>();
                if related_jobs
                    .iter()
                    .any(|x| x.status() == TransferJobStatus::Running)
                {
                    println!("Job is currently running. Skipping...");
                    continue;
                }
                let now = Utc::now();
                let next_run = related_jobs
                    .iter()
                    .filter_map(|x| x.end_date)
                    .max()
                    .map(|last_ran| {
                        cron_schedule
                            .after(&last_ran.and_utc())
                            .next()
                            .ok_or_else(|| {
                                ApiError::InternalError("Failed to calculate next run!".into())
                            })
                    })
                    .transpose()? // unwraps Result<Option<T>> to Option<T>, propagates Err
                    .unwrap_or(now);

                if now >= next_run
                    || start_jobs.iter().any(|x| {
                        job_schedule.server_name == x.server_name
                            && job_schedule.schedule_name == x.schedule_name
                    })
                {
                    self.create_and_start_job(&RCloneJobLaunchInfo {
                        server_name: job.server_name.clone(),
                        smb_user_name: job_schedule.smb_user_name.clone(),
                        smb_password: job_schedule.smb_password.clone(),
                        local_folder: job.local_folder.clone(),
                        smb_folder: job.smb_folder.clone(),
                        auto_job: Some(job.action_name.clone()),
                    })?;
                }
            }
        }
        // For each job, attempt to start it in a batch if possible.
        Ok(())
    }

    pub fn handle_blocking(&self) {
        let (tx, rx) = channel::<PostForAutoScheduleStartDto>();
        {
            let y = &mut *self.start_tx.lock().unwrap();
            *y = Some(tx);
        }
        loop {
            // We need to pull all jobs and see if they are started.
            if let Err(e) = self._thread_iter(&rx) {
                println!("Failed to run the Auto Job portion. Error: {e}");
            } else {
                println!("Successfully ran Auto Job portion.");
            }
            thread::sleep(Duration::from_secs(30));
        }
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

    fn _parse_smb_path(smb_user_name: &str, smb_folder: &str) -> Option<String> {
        // Extract the username by stripping "-smb"
        let started_by = smb_user_name.strip_suffix("-smb")?;

        use std::path::Component;
        use std::path::Path;
        use std::path::PathBuf;

        let path = Path::new(smb_folder);
        let is_absolute = path.is_absolute();

        // Extract "normal" components (skip root, curdir, etc.)
        let segments: Vec<_> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                _ => None,
            })
            .collect();

        // Require at least two segments
        if segments.len() < 2 {
            return None;
        }

        let first = &segments[0];
        let second = &segments[1];

        // Only proceed if second component is "data" or "repo"
        match second.as_str() {
            "data" | "repo" => {
                // Form the new prefix: "user-first-second"
                let new_prefix = format!("{}-{}-{}", started_by, first, second);

                let mut result_path = PathBuf::new();
                if is_absolute {
                    result_path.push("/");
                }

                result_path.push(new_prefix);

                // Append any remaining path segments
                for segment in &segments[2..] {
                    result_path.push(segment);
                }

                Some(result_path.to_string_lossy().to_string())
            }
            _ => None,
        }
    }

    ////////////////////////////////////////////////// all public methods below

    //noinspection RsFormatMacroWithoutFormatArguments
    pub fn start_job(&self, job_id: Uuid) -> Result<(), ApiError> {
        self._ensure_check()?; // **** make sure we are okay!
        let _lock = &mut *self.jobs.lock().unwrap();
        let job = _lock
            .iter_mut()
            .filter(|x| x.status() != TransferJobStatus::Running)
            .find(|x| x.job_id == job_id)
            .ok_or(ApiError::BadRequest(
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

        let out_folder = Self::_parse_smb_path(&job.smb_user_name, &job.smb_folder).ok_or(
            ApiError::BadRequest("You did not correctly put in the SMB folder!".into()),
        )?;
        let cmd_exp = cmd!(
            exe_path_str,
            "sync",
            &job.local_folder,
            format!("{}:{}", host_id, &out_folder),
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

    pub fn cancel_job(&self, job_id: Uuid) -> Result<(), ApiError> {
        let _lock = &mut *self.jobs.lock().unwrap();
        let job = _lock
            .iter_mut()
            .filter(|x| x.status() == TransferJobStatus::Running)
            .find(|x| x.job_id == job_id)
            .ok_or(ApiError::BadRequest(
                "Job ID does not exist or is not running!".into(),
            ))?;
        let tx = job._cancel_tx.as_ref().ok_or(ApiError::InternalError(
            "Cancel is not supported (no TX)".into(),
        ))?;
        let rx = job._cancel_rx.as_ref().ok_or(ApiError::InternalError(
            "Cancel is not supported (no RX)".into(),
        ))?;
        tx.send(())
            .map_err(|_| ApiError::InternalError("Failed to send request!".into()))?;
        rx.recv_timeout(Duration::from_secs(3))
            .ok()
            .filter(|&x| x)
            .map(|_| ())
            .ok_or(ApiError::InternalError(
                "Timeout exceeded or failed to cancel!".into(),
            ))
    }

    pub fn create_and_start_job(&self, info: &RCloneJobLaunchInfo) -> Result<Uuid, ApiError> {
        let id = self.create_job(info)?;
        self.start_job(id)?;
        Ok(id)
    }

    pub fn create_job(&self, info: &RCloneJobLaunchInfo) -> Result<Uuid, ApiError> {
        let job_id = Uuid::new_v4();
        let all_servers = self.db.get_all_servers_sync().map_err(|x| {
            ApiError::InternalError(format!("Failed to pull all servers: {}", x.to_string()))
        })?;
        // Attempt to pull the Server Name from the DB Controller first.
        let info = info
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let server = all_servers
            .iter()
            .find(|x| x.server_name == info.server_name)
            .map(|x| x.to_owned())
            .ok_or(ApiError::BadRequest("Cannot locate server!".into()))?;
        {
            let y = &mut *self.jobs.lock().unwrap();
            y.push(TransferJob {
                server,
                job_id: job_id.clone(),
                smb_user_name: info.smb_user_name,
                smb_password: info.smb_password,
                smb_folder: info.smb_folder,
                local_folder: info.local_folder,
                auto_job: info.auto_job,
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
        Ok(job_id)
    }

    pub fn get_job(&self, job_id: Uuid) -> Option<TransferJobDto> {
        let _lock = &*self.jobs.lock().unwrap();
        _lock.iter().find(|x| x.job_id == job_id).map(|x| x.into())
    }

    pub fn new(
        settings: RCloneSettings,
        jobs: Arc<Mutex<Vec<TransferJob>>>,
        db: Arc<DbController>,
    ) -> Self {
        Self {
            settings,
            jobs,
            db,
            start_tx: Mutex::new(None),
        }
    }

    pub fn new_owned(settings: RCloneSettings, db: Arc<DbController>) -> Self {
        Self::new(settings, Arc::new(Mutex::new(vec![])), db)
    }
}
