use crate::apis::NeptisError;
use crate::db::sync_models::{
    RCloneMessage, RCloneStat, TransferJob, TransferJobDto, TransferJobStatus,
};
use crate::ipc::errors::ApiError;
use crate::prelude::{
    DbController, JobStatus, PostForAutoScheduleStartDto, TransferJobInternalDto, WebApi,
};
use crate::rolling_secret::RollingSecret;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::Utc;
use cron::Schedule;
use duct::cmd;
use merkle_hash::{Algorithm, MerkleTree};
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, MutexGuard};
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
    pub auto_job_schedule_name: Option<String>,
    pub auto_job_action_name: Option<String>,
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
}

pub struct RCloneClient {
    settings: RCloneSettings,
    db: Arc<DbController>,
    start_tx: Mutex<Option<Sender<PostForAutoScheduleStartDto>>>,
    _jobs: Arc<Mutex<Vec<TransferJob>>>,
    rt: Arc<Runtime>,
}

impl RCloneClient {
    fn _get_jobs_locked(&self) -> Result<MutexGuard<Vec<TransferJob>>, ApiError> {
        let mut _lock = self._jobs.lock().unwrap();
        {
            let saved_jobs = &mut *_lock;
            for db_job in self.db.get_all_transfer_jobs_internal_sync()? {
                if !saved_jobs.iter().any(|x| x.dto.job_id == db_job.job_id) {
                    saved_jobs.push(db_job.into());
                }
            }
        }
        Ok(_lock)
    }

    async fn _get_jobs_async(&self) -> Result<Vec<TransferJobDto>, ApiError> {
        let db_jobs = self.db.get_all_transfer_jobs_internal().await?;
        let all_jobs = {
            let mut _lock = self._jobs.lock().unwrap();
            let saved_jobs = &mut *_lock;
            for db_job in db_jobs {
                if !saved_jobs.iter().any(|x| x.dto.job_id == db_job.job_id) {
                    saved_jobs.push(db_job.into());
                }
            }
            saved_jobs.iter().map(|x| x.into()).collect::<Vec<_>>()
        };
        Ok(all_jobs)
    }

    fn _save_jobs(&self) -> Result<(), ApiError> {
        for job in &*self._jobs.lock().unwrap() {
            self.db.save_transfer_job_internal_sync(&job.dto)?
        }
        Ok(())
    }

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
        println!("*** Thread loop beginning...");
        let mut start_jobs = vec![];
        loop {
            match start_rx.try_recv() {
                Ok(x) => start_jobs.push(x),
                Err(_) => break,
            }
        }
        println!("> Found {} immediate start jobs from TX", start_jobs.len());
        let all_schedules = self.db.get_all_transfer_auto_schedules_sync()?;
        let mut all_infos = vec![];
        {
            // We need to keep the `all_jobs` variable in a separate scope due to locking.
            let all_jobs = self._get_jobs_locked()?;
            for job in self.db.get_all_transfer_auto_jobs_sync()? {
                if !job.enabled {
                    continue;
                }
                println!(
                    "> Checking {}/{}/{}",
                    job.server_name, job.schedule_name, job.action_name
                );
                // Find the schedule and see if a job is currently running or not.
                if let Some((job_schedule, cron_schedule)) = all_schedules
                    .iter()
                    .find(|x| {
                        x.schedule_name == job.schedule_name && x.server_name == job.server_name
                    })
                    .map(|x| Schedule::from_str(&x.cron_schedule).ok().map(|y| (x, y)))
                    .flatten()
                {
                    let related_jobs = all_jobs
                        .iter()
                        .filter(|x| {
                            x.dto.server_name == job.server_name
                                && x.dto.auto_job_action_name == Some(job.action_name.clone())
                                && x.dto.auto_job_schedule_name == Some(job.schedule_name.clone())
                        })
                        .collect::<Vec<_>>();
                    if related_jobs
                        .iter()
                        .any(|x| x.status() == TransferJobStatus::Running)
                    {
                        println!("> Job is currently running. Skipping...");
                        continue;
                    }
                    let now = Utc::now();
                    let last_ran = related_jobs
                        .iter()
                        .filter_map(|x| x.dto.end_date.or(x.dto.start_date))
                        .max()
                        .map(|dt| dt.and_utc())
                        .unwrap_or(job_schedule.last_updated.and_utc());

                    let next_run = cron_schedule.after(&last_ran).next();
                    println!("> Last ran = {}", last_ran);

                    let mut do_run = false;
                    if let Some(next_run) = next_run {
                        println!("> Next run = {}", next_run);
                        do_run = now >= next_run;
                    } else {
                        println!("> Next run = NONE");
                    }

                    if do_run
                        || start_jobs.iter().any(|x| {
                            job_schedule.server_name == x.server_name
                                && job_schedule.schedule_name == x.schedule_name
                        })
                    {
                        println!("> Run is desired. Adding to start list...");
                        all_infos.push(RCloneJobLaunchInfo {
                            server_name: job.server_name.clone(),
                            smb_user_name: job_schedule.smb_user_name.clone(),
                            smb_password: job_schedule.smb_password.clone(),
                            local_folder: job.local_folder.clone(),
                            smb_folder: job.smb_folder.clone(),
                            auto_job_schedule_name: Some(job.schedule_name.clone()),
                            auto_job_action_name: Some(job.action_name.clone()),
                        });
                    } else {
                        println!("> Not meeting run schedule. Skipping...");
                    }
                }
            }
        }

        for info in all_infos {
            self.create_and_start_job(&info)?;
        }

        self._save_jobs() // *** always save all jobs at the end!
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
        db: Arc<DbController>,
        rt: Arc<Runtime>,
        s_rx: Receiver<()>,
        r_tx: Sender<bool>,
        is_same: bool,
    ) {
        let mark_message = |msg: &str, fatal: bool, stat: Option<RCloneStat>| {
            let _lock = &mut *jobs.lock().unwrap();
            let current_now = Utc::now().naive_utc();
            let job = _lock
                .iter_mut()
                .find(|x| x.dto.job_id == job_id)
                .expect("Expected job to exist after creation!");
            if !msg.is_empty() {
                if fatal {
                    if !job.dto.fatal_errors.contains(&msg.into()) {
                        job.dto.fatal_errors.push(msg.into());
                        println!("Adding error '{}' to {}", msg, job.dto.job_id);
                    }
                } else {
                    if !job.dto.warnings.contains(&msg.into()) {
                        job.dto.warnings.push(msg.into());
                        println!("Adding warning '{}' to {}", msg, job.dto.job_id);
                    }
                }
            }
            if let Some(stat) = stat {
                job.dto.last_stats = Some(stat.into()); // *** prevent a None value from overwriting.
            }
            if fatal {
                // We can assume the thread has already been disconnected here.
                job.dto.end_date = Some(current_now);
                job._thread = None;
                job._cancel_tx = None;
                job._cancel_rx = None;
                println!("> Ending job {}", job.dto.job_id);
            }
            job.dto.last_updated = current_now;
            let _ = db.save_transfer_job_internal_sync(&job.dto);
        };

        let server_item = {
            let server_name = {
                let _lock = &mut *jobs.lock().unwrap();
                _lock
                    .iter_mut()
                    .find(|x| x.dto.job_id == job_id)
                    .expect("Expected job to exist after creation!")
                    .dto
                    .server_name
                    .clone()
            };
            db.get_all_servers_sync()
                .ok()
                .and_then(|x| x.into_iter().find(|x| x.server_name == server_name))
        };

        if is_same {
            // The job is the same - we don't need to do anything!
            thread::sleep(Duration::from_secs(2));
            mark_message(
                "Files appear to be the same as last time. Skipping transfer...",
                false,
                None,
            );
            mark_message("", true, None);
            return;
        }

        // If the server item exists, attempt to turn it on.
        let job_info = {
            let _lock = &mut *jobs.lock().unwrap();
            let job = _lock
                .iter()
                .find(|x| x.dto.job_id == job_id)
                .expect("Expected job to exist after creation!");
            if let Some(ref schedule_name) = job.dto.auto_job_schedule_name {
                db.get_all_transfer_auto_schedules_sync()
                    .ok()
                    .and_then(|x| {
                        x.into_iter().find(|x| {
                            x.server_name == job.dto.server_name
                                && x.schedule_name == *schedule_name
                        })
                    })
                    .map(|x| (x, job.dto.smb_folder.clone()))
            } else {
                None
            }
        };

        // Attempt to find a username and password combo to test with.
        if let Some(ref server_item) = server_item {
            if let Some((test_name, test_pass)) = {
                if let Some(test_name) = server_item.user_name.clone()
                    && let Some(test_pass) = server_item.user_password.clone()
                {
                    Some((test_name, test_pass))
                } else if let Some((auto_schedule, _)) = job_info.clone()
                    && let Some(test_pass) = auto_schedule.user_password
                    && !auto_schedule.smb_user_name.is_empty()
                {
                    let test_name = auto_schedule
                        .smb_user_name
                        .trim_end_matches("-smb")
                        .to_string();
                    Some((test_name, test_pass))
                } else {
                    None
                }
            } {
                // Attempt to wake up the server with the Arduino - then connect.
                if let Some(arduino_ep) = server_item.arduino_endpoint.clone()
                    && let Some(arduino_pass) = server_item.arduino_password.clone()
                {
                    for _ in 0..2 {
                        if rt
                            .block_on(async { WebApi::wake_pc(&arduino_ep, &arduino_pass).await })
                            .is_ok()
                        {
                            break;
                        }
                        thread::sleep(Duration::from_secs(2));
                    }
                }
                let mut res = Err(NeptisError::Str("Failed to connect to server!".into()));
                for _ in 0..2 {
                    let api = WebApi::new(
                        &server_item.server_endpoint,
                        &test_name,
                        &test_pass,
                        server_item
                            .server_password
                            .clone()
                            .and_then(|x| RollingSecret::from_string(&x)),
                    );
                    res = rt.block_on(async move { api.get_info().await });
                    if res.is_ok() {
                        break;
                    }
                    thread::sleep(Duration::from_secs(2));
                }
                if let Err(e) = res {
                    mark_message(&e.to_string(), true, None);
                    return;
                }
            }
        }

        match cmd.reader() {
            Ok(handle) => {
                let rdr = BufReader::new(&handle);
                for line in rdr.lines() {
                    if let Ok(line) = line {
                        println!("{}", &line);
                        let trimmed = line.trim_matches('"');
                        let unescaped = trimmed.replace("\\\"", "\"");
                        println!();
                        match serde_json::from_str::<RCloneMessage>(&unescaped) {
                            Ok(msg) => {
                                mark_message("", false, msg.stats);
                            }
                            Err(e) => println!("Json Error: {e}"),
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

                // Attempt to check if the backup should be finished.
                if let Some((auto_schedule, smb_path)) = job_info.clone()
                    && let Some(user_pass) = auto_schedule.user_password
                    && let Some(point_name) = smb_path
                        .trim_start_matches("/")
                        .split_once("/")
                        .map(|x| x.0.to_string())
                    && let Some(server_item) = server_item.clone()
                    && auto_schedule.backup_on_finish
                {
                    let user_name = auto_schedule
                        .smb_user_name
                        .trim_end_matches("-smb")
                        .to_string();
                    let api = WebApi::new(
                        &server_item.server_endpoint,
                        &user_name,
                        &user_pass,
                        server_item
                            .server_password
                            .clone()
                            .and_then(|x| RollingSecret::from_string(&x)),
                    );
                    match rt.block_on(async {
                        api.post_one_backup(&point_name, false).await.map(|x| x.id)
                    }) {
                        Ok(b_id) => {
                            // Attempt to read the status from the server until completion.
                            thread::sleep(Duration::from_secs(2)); // *** wait for server init.
                            loop {
                                if let Ok(b_job) =
                                    rt.block_on(async { api.get_one_job(b_id).await })
                                {
                                    let stat = RCloneStat {
                                        bytes: b_job.used_bytes as u64,
                                        speed: 0,
                                        checks: 0,
                                        deletes: 0,
                                        listed: 0,
                                        renames: 0,
                                        retry_error: false,
                                        deleted_dirs: 0,
                                        server_side_copies: 0,
                                        server_side_copy_bytes: 0,
                                        server_side_move_bytes: 0,
                                        server_side_moves: 0,
                                        total_bytes: b_job.total_bytes.unwrap_or(0) as u64,
                                        total_checks: 0,
                                        total_transfers: 0,
                                        on_backup: true,
                                    };
                                    mark_message("", false, Some(stat));
                                    for error in b_job.errors.iter() {
                                        mark_message(
                                            &format!("Backup Job: {}", error),
                                            false,
                                            None,
                                        );
                                    }

                                    if b_job.job_status == JobStatus::Failed {
                                        mark_message("Backup Job reported a failure.", true, None);
                                        return;
                                    } else if b_job.job_status == JobStatus::Successful {
                                        break;
                                    } else {
                                        thread::sleep(Duration::from_secs(5));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            mark_message(&format!("Failed to start backup: {}", e), false, None);
                        }
                    }
                }

                mark_message("", true, None); // *** job has finished!
            }
            Err(e) => {
                let err = &format!("Failed to pull reader! Error: {}", e);
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

    fn _remove_old_tmp_files<P: AsRef<Path>>(folder: P, init: bool) {
        let one_day = Duration::from_secs(60 * 60 * 24);
        let now = SystemTime::now();

        if let Ok(entries) = fs::read_dir(&folder) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if path.extension().map_or(false, |ext| ext == "tmp") {
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                if init
                                    || now.duration_since(modified).unwrap_or_default() > one_day
                                {
                                    println!("Deleting: {:?}", path);
                                    let _ = fs::remove_file(&path); // Ignore errors
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ////////////////////////////////////////////////// all public methods below

    //noinspection RsFormatMacroWithoutFormatArguments
    pub fn start_job(&self, job_id: Uuid) -> Result<(), ApiError> {
        self._ensure_check()?; // **** make sure we are okay!
        let mut _lock = self._get_jobs_locked()?;

        let last_hash = {
            if let Some(current_job) = _lock
                .iter()
                .find(|x| x.dto.job_id == job_id)
                .map(|x| x.dto.clone())
            {
                _lock
                    .iter()
                    .filter(|x| {
                        x.dto.auto_job_action_name == current_job.auto_job_action_name
                            && x.dto.auto_job_schedule_name == current_job.auto_job_schedule_name
                            && x.dto.server_name == current_job.server_name
                            && x.dto.local_folder == current_job.local_folder
                    })
                    .max_by(|a, b| a.dto.end_date.cmp(&b.dto.end_date))
                    .and_then(|x| x.dto.init_hash.clone())
            } else {
                None
            }
        };

        let job = _lock
            .iter_mut()
            .filter(|x| x.status() != TransferJobStatus::Running)
            .find(|x| x.dto.job_id == job_id)
            .ok_or(ApiError::BadRequest(
                "Job ID does not exist or is running already!".into(),
            ))?;

        let server = self
            .db
            .get_all_servers_sync()?
            .into_iter()
            .find(|x| x.server_name == job.dto.server_name)
            .ok_or(ApiError::BadRequest("Server does not exist!".into()))?;

        // Attempt to check if the hash matches an existing one.
        job.dto.init_hash = MerkleTree::builder(&job.dto.local_folder)
            .algorithm(Algorithm::Sha256)
            .hash_names(true) // *** include file names!
            .build()
            .map(|x| BASE64_STANDARD.encode(x.root.item.hash))
            .ok();

        let is_same = if let Some(ref old_hash) = last_hash
            && let Some(ref new_hash) = job.dto.init_hash
            && old_hash == new_hash
        {
            true
        } else {
            false
        };

        let exe_path = self.settings.exe_path();
        let exe_path_str = exe_path.to_str().unwrap();

        let host_id = BASE64_STANDARD.encode(&server.server_name).replace("=", "");
        let mut c_entry = String::new();
        {
            let host = Self::_find_smb_address(&server.server_endpoint)?;
            let pass = cmd!(exe_path_str, "obscure", &job.dto.smb_password).read()?;
            c_entry += &format!("[{}]\n", &host_id);
            c_entry += &format!("type = smb\n");
            c_entry += &format!("host = {}\n", host);
            c_entry += &format!("user = {}\n", &job.dto.smb_user_name);
            c_entry += &format!("pass = {}\n", pass);
        }

        Self::_remove_old_tmp_files(&self.settings.working_path, false);
        let mut config_path = self.settings.working_path.join(Uuid::new_v4().to_string());
        config_path.set_extension("tmp");
        fs::write(&config_path, c_entry)?;

        let out_folder = Self::_parse_smb_path(&job.dto.smb_user_name, &job.dto.smb_folder).ok_or(
            ApiError::BadRequest("You did not correctly put in the SMB folder!".into()),
        )?;
        let cmd_exp = cmd!(
            exe_path_str,
            "sync",
            &job.dto.local_folder,
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
        let jobs = self._jobs.clone();
        let rt = self.rt.clone();
        let db = self.db.clone();
        let (s_tx, s_rx) = channel::<()>();
        let (r_tx, r_rx) = channel::<bool>();
        job.dto.start_date = Some(Utc::now().naive_utc());
        job.dto.fatal_errors = vec![].into();
        job.dto.warnings = vec![].into();
        job.dto.last_stats = None;
        job._cancel_tx = Some(s_tx);
        job._cancel_rx = Some(r_rx);
        job._thread = Some(thread::spawn(move || {
            Self::_handle_job(job_id, cmd_exp, jobs, db, rt, s_rx, r_tx, is_same);
        }));
        Ok(())
    }

    pub fn start_auto_job(&self, info: PostForAutoScheduleStartDto) -> Result<(), ApiError> {
        let _lock = &*self.start_tx.lock().unwrap();
        if let Some(tx) = _lock {
            tx.send(info)
                .map_err(|e| ApiError::InternalError(e.to_string()))
        } else {
            Err(ApiError::InternalError(
                "IPC starter is not running!".into(),
            ))
        }
    }

    pub fn cancel_job(&self, job_id: Uuid) -> Result<(), ApiError> {
        let mut _lock = self._get_jobs_locked()?;
        let job = _lock
            .iter_mut()
            .filter(|x| x.status() == TransferJobStatus::Running)
            .find(|x| x.dto.job_id == job_id)
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
            let mut y = self._get_jobs_locked()?;
            y.push(TransferJob {
                dto: TransferJobInternalDto {
                    job_id,
                    auto_job_action_name: info.auto_job_action_name,
                    auto_job_schedule_name: info.auto_job_schedule_name,
                    server_name: server.server_name,
                    smb_user_name: info.smb_user_name.clone(),
                    smb_password: info.smb_password.clone(),
                    smb_folder: info.smb_folder.clone(),
                    local_folder: info.local_folder.clone(),
                    last_stats: None,
                    start_date: None,
                    end_date: None,
                    fatal_errors: vec![].into(),
                    warnings: vec![].into(),
                    last_updated: Utc::now().naive_utc(),
                    init_hash: None, // *** do not calculate until starting
                },
                _thread: None,
                _cancel_tx: None,
                _cancel_rx: None,
            })
        }
        Ok(job_id)
    }

    pub async fn get_job(&self, job_id: Uuid) -> Option<TransferJobDto> {
        let _lock = self._get_jobs_async().await.ok()?;
        _lock.into_iter().find(|x| x.job_id == job_id)
    }

    pub async fn get_all_jobs(&self) -> Option<Vec<TransferJobDto>> {
        self._get_jobs_async().await.ok()
    }

    pub fn new(settings: RCloneSettings, db: Arc<DbController>, rt: Arc<Runtime>) -> Self {
        let cli = Self {
            settings,
            db,
            start_tx: Mutex::new(None),
            _jobs: Arc::new(Mutex::new(vec![])),
            rt,
        };
        Self::_remove_old_tmp_files(&cli.settings.working_path, true);
        cli
    }
}

impl Drop for RCloneClient {
    fn drop(&mut self) {
        // Attempt to automatically clean the files upon closing.
        Self::_remove_old_tmp_files(&self.settings.working_path, true);
    }
}
