#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate core;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

use axoupdater::{
    AxoUpdater, AxoupdateError, ReleaseSource, ReleaseSourceType, UpdateRequest, Version,
};
use chrono::{Local, Utc};
use cron::{Schedule, TimeUnitSpec};
use inquire::list_option::ListOption;
use inquire::{Editor, MultiSelect};
use reqwest::ClientBuilder;
use std::ffi::OsStr;
use std::iter::once;
use std::process;
use std::str::FromStr;
use uuid::Uuid;

use inquire::{Confirm, CustomType, Password, Select, Text, required, validator::Validation};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::RwLock;
use std::thread::JoinHandle;
use std::time::Instant;
use std::{
    fs,
    ops::DerefMut,
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Serialize, Deserialize, Clone, Default)]
struct InternalMountDto {
    name: String,
    data_bytes: i64,
    repo_bytes: i64,
}

impl ToShortIdString for InternalMountDto {
    fn to_short_id_string(&self) -> String {
        self.name.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct InternalTransferAutoSchedule {
    schedule_name: String,
    cron_schedule: String,
    smb_password: String,
    backup_on_finish: bool,
}

impl ToShortIdString for InternalTransferAutoSchedule {
    fn to_short_id_string(&self) -> String {
        self.schedule_name.clone()
    }
}

impl From<TransferAutoSchedule> for InternalTransferAutoSchedule {
    fn from(value: TransferAutoSchedule) -> Self {
        InternalTransferAutoSchedule {
            schedule_name: value.schedule_name,
            cron_schedule: value.cron_schedule,
            smb_password: value.smb_password,
            backup_on_finish: value.backup_on_finish,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct InternalTransferAutoJob {
    action_name: String,
    smb_folder: String,
    local_folder: String,
    enabled: bool,
}

impl ToShortIdString for InternalTransferAutoJob {
    fn to_short_id_string(&self) -> String {
        self.action_name.clone()
    }
}

impl From<TransferAutoJob> for InternalTransferAutoJob {
    fn from(value: TransferAutoJob) -> Self {
        InternalTransferAutoJob {
            action_name: value.action_name,
            smb_folder: value.smb_folder,
            local_folder: value.local_folder,
            enabled: value.enabled,
        }
    }
}

#[cfg(unix)]
struct UiApp {
    rt: Arc<Runtime>,
    api: Arc<RwLock<Option<WebApi>>>,
    fuse: Mutex<Option<fuser::BackgroundSession>>,
    db: DbController,
    mnt: Option<String>,
    server: RwLock<Option<ServerItem>>,
}

#[cfg(not(unix))]
struct UiApp {
    rt: Arc<Runtime>,
    api: Arc<RwLock<Option<WebApi>>>,
    db: DbController,
    server: RwLock<Option<ServerItem>>,
}

static DEFAULT_PASS: &'static str = "default123";
static MAX_JOBS: usize = 5;
static MAX_SYNC_JOBS: usize = 5;

impl UiApp {
    // inspected
    fn on_select_snapshot(&self, mount: &str, u_snapshots: &[SnapshotFileDto]) {
        use crossterm::{
            event::{self, Event, KeyCode},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::io::{Write, stdout};
        fn format_duration_hms(seconds: f64) -> String {
            let total_seconds = seconds.round() as u64;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let secs = total_seconds % 60;

            match (hours, minutes, secs) {
                (0, 0, s) => format!("{s}s"),
                (0, m, s) => format!("{m}m {s}s"),
                (h, m, s) => format!("{h}h {m}m {s}s"),
            }
        }

        let mut index = 0;
        let mut snapshots = u_snapshots.to_vec();
        let len = snapshots.len();
        loop {
            clearscreen::clear().expect("Failed to clear screen!");
            let snapshot = &mut snapshots[index];
            println!(
                "================= Snapshot Info ({}/{}) =================",
                index + 1,
                len
            );
            println!("ID:               {}", snapshot.id);
            println!(
                "Locked:           {}",
                if snapshot.locked { "YES" } else { "NO" }
            );
            println!("Label:            {}", snapshot.label);
            println!("Tree:             {}", snapshot.tree);
            println!("Program Version:  {}", snapshot.program_version);
            println!(
                "Parent Snapshot:  {}",
                snapshot.parent.as_deref().unwrap_or("-")
            );
            println!(
                "Original:         {}",
                snapshot.original.as_deref().unwrap_or("-")
            );
            println!(
                "Description:      {}",
                snapshot.description.as_deref().unwrap_or("-")
            );
            println!(
                "Paths:            {}",
                if snapshot.paths.is_empty() {
                    "-".to_string()
                } else {
                    snapshot.paths.join(", ")
                }
            );
            println!(
                "Tags:             {}",
                if snapshot.tags.is_empty() {
                    "-".to_string()
                } else {
                    snapshot.tags.join(", ")
                }
            );

            if let Some(summary) = &snapshot.summary {
                println!("\n----------- Snapshot Summary -----------");
                println!("New Files:              {}", summary.files_new);
                println!("Changed Files:          {}", summary.files_changed);
                println!("Unmodified Files:       {}", summary.files_unmodified);
                println!("Total Files Processed:  {}", summary.total_files_processed);
                println!("Total Bytes Processed:  {}", summary.total_bytes_processed);
                println!("New Dirs:               {}", summary.dirs_new);
                println!("Changed Dirs:           {}", summary.dirs_changed);
                println!("Unmodified Dirs:        {}", summary.dirs_unmodified);
                println!("Total Dirs Processed:   {}", summary.total_dirs_processed);
                println!(
                    "Total Dir Size:         {}",
                    summary.total_dirsize_processed
                );
                println!("Data Blobs:             {}", summary.data_blobs);
                println!("Tree Blobs:             {}", summary.tree_blobs);
                println!("Data Added (raw):       {}", summary.data_added);
                println!("Data Added (packed):    {}", summary.data_added_packed);
                println!("Files Added (raw):      {}", summary.data_added_files);
                println!(
                    "Files Added (packed):   {}",
                    summary.data_added_files_packed
                );
                println!("Trees Added (raw):      {}", summary.data_added_trees);
                println!(
                    "Trees Added (packed):   {}",
                    summary.data_added_trees_packed
                );
                println!("Command:                {}", summary.command);
                println!(
                    "Backup Start:           {}",
                    summary.backup_start.format("%Y-%m-%d %H:%M:%S")
                );
                println!(
                    "Backup End:             {}",
                    summary.backup_end.format("%Y-%m-%d %H:%M:%S")
                );
                println!(
                    "Backup Duration:        {:.2} sec ({})",
                    summary.backup_duration,
                    format_duration_hms(summary.backup_duration)
                );
                println!(
                    "Total Duration:         {:.2} sec ({})",
                    summary.total_duration,
                    format_duration_hms(summary.total_duration)
                );
            } else {
                println!("\n(No snapshot summary available)");
            }

            println!("\n←/→ to browse | l to toggle lock | q or Enter to go back");

            enable_raw_mode().expect("Failed to enable raw mode");
            let result = event::read();
            disable_raw_mode().expect("Failed to disable raw mode");

            match result {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Left => {
                        if key.is_press() && index > 0 {
                            index -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if key.is_press() && index + 1 < snapshots.len() {
                            index += 1;
                        }
                    }
                    KeyCode::Char('l') => {
                        // Attempt to lock or unlock the given snapshot.
                        if key.is_press() {
                            println!("*** Please wait...");
                            let m_api = &*self.api.read().unwrap();
                            let is_locked = snapshot.locked;
                            let id = snapshot.id.clone();
                            if {
                                if let Some(api) = m_api {
                                    self.rt.block_on(async move {
                                        if !is_locked {
                                            api.lock_one_snapshot(mount, id.as_str()).await
                                        } else {
                                            api.unlock_one_snapshot(mount, id.as_str()).await
                                        }
                                    })
                                } else {
                                    Err(NeptisError::Str("API is invalid!".into()))
                                }
                            }
                            .is_ok()
                            {
                                snapshot.locked = !is_locked;
                                continue;
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Enter => {
                        if key.is_press() {
                            break;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        self.on_manage_snapshot(mount);
    }

    fn on_select_rclone_job(&self, id: Uuid, schedule_name: &str, server_name: &str) {
        let mut last_refresh = Instant::now();
        let mut first_time = true;
        loop {
            if first_time || last_refresh.elapsed().as_secs() >= 5 {
                first_time = false;
                clearscreen::clear().expect("Failed to clear screen!");
                match self.rt.block_on(async { WebApi::ipc_get_job(id).await }) {
                    Ok(dto) => {
                        println!("================ Transfer Job Info ================");
                        println!("Job ID:             {}", dto.job_id);
                        println!("Server Name:        {}", dto.server_name);
                        println!("SMB Folder:         {}", dto.smb_folder);
                        println!("Local Folder:       {}", dto.local_folder);
                        println!("Status:             {:?}", dto.stat);

                        if !dto.errors.is_empty() {
                            println!(
                                "Errors ({}):         {}",
                                dto.errors.len(),
                                dto.errors.join(", ")
                            );
                        } else {
                            println!("Errors:             -");
                        }

                        println!(
                            "Start Date:         {}",
                            dto.start_date
                                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "-".to_string())
                        );
                        println!(
                            "End Date:           {}",
                            dto.end_date
                                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "-".to_string())
                        );
                        println!(
                            "Last Updated:       {}",
                            dto.last_updated.format("%Y-%m-%d %H:%M:%S")
                        );

                        if let Some(stats) = &dto.last_stats {
                            println!("------------------ Last RClone Stats --------------");
                            println!("Transferred:        {}", FileSize::prettify(stats.bytes));
                            println!("Transfer Speed:     {}/s", FileSize::prettify(stats.speed));
                            println!("Checks:             {}", stats.checks);
                            println!("Deletes:            {}", stats.deletes);
                            println!("Renames:            {}", stats.renames);
                            println!("Listed:             {}", stats.listed);
                            println!("Retry Error:        {}", stats.retry_error);
                            println!("Deleted Dirs:       {}", stats.deleted_dirs);
                            println!("Server Copies:      {}", stats.server_side_copies);
                            println!(
                                "Copy Bytes:         {}",
                                FileSize::prettify(stats.server_side_copy_bytes)
                            );
                            println!(
                                "Move Bytes:         {}",
                                FileSize::prettify(stats.server_side_move_bytes)
                            );
                            println!("Moves:              {}", stats.server_side_moves);
                            println!(
                                "Total Bytes:        {}",
                                FileSize::prettify(stats.total_bytes)
                            );
                            println!("Total Checks:       {}", stats.total_checks);
                            println!("Total Transfers:    {}", stats.total_transfers);
                        } else {
                            println!("Last RClone Stats:  -");
                        }

                        println!("===================================================\n");
                        println!("(Waiting... Press any key to select options/exit)\n");
                    }
                    Err(e) => {
                        println!("Failed to show job information: {:?}", e);
                        break;
                    }
                }
                last_refresh = Instant::now();
            }

            if event::poll(Duration::from_millis(100)).unwrap() {
                match event::read().unwrap() {
                    Event::Key(key_event) => {
                        if key_event.is_press() {
                            if key_event.code == KeyCode::Enter {
                                break;
                            }

                            // Go to interactive menu
                            const STR_CANCEL: &'static str = "Cancel Job";
                            const STR_GO_BACK: &'static str = "Go Back";
                            let mut options = vec![STR_GO_BACK];
                            if let Ok(job) =
                                self.rt.block_on(async { WebApi::ipc_get_job(id).await })
                                && job.stat == TransferJobStatus::Running
                            {
                                options.push(STR_CANCEL);
                            }

                            if Select::new("Please select an action", options)
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                                == Some(STR_CANCEL)
                            {
                                println!("\n*** Cancelling job...");
                                match self.rt.block_on(async { WebApi::ipc_cancel_job(id).await }) {
                                    Ok(_) => println!("  Successful!"),
                                    Err(e) => println!("  Failed! Error: {e}"),
                                }
                                first_time = true; // *** force an instant refresh!
                                thread::sleep(Duration::from_secs(2));
                                continue;
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.on_manage_rclone_jobs(schedule_name, server_name);
    }

    // inspected
    fn on_select_job(&self, mount: &str, j_id: Uuid, point: Option<String>) {
        use crossterm::{
            event::{self, Event, KeyCode},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::{
            process,
            time::{Duration, Instant},
        };
        let mut last_refresh = Instant::now();
        let mut first_time = true;
        loop {
            // Refresh job details every 5 seconds
            if first_time || last_refresh.elapsed().as_secs() >= 5 {
                first_time = false;
                clearscreen::clear().expect("Failed to clear screen!");
                let result = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        self.rt.block_on(async move {
                            api.get_all_jobs_for_mount(mount)
                                .await?
                                .into_iter()
                                .find(|x| x.id == j_id)
                                .ok_or(NeptisError::Str("Failed to find the job!".into()))
                        })
                    } else {
                        Err(NeptisError::Str("API is not valid!".into()))
                    }
                };

                match result {
                    Ok(dto) => {
                        println!("================= Repo Job Info =================");
                        println!("ID:                {}", dto.id);
                        println!("Title:             {}", dto.title.as_deref().unwrap_or("-"));
                        println!(
                            "Snapshot ID:       {}",
                            dto.snapshot_id.as_deref().unwrap_or("-")
                        );
                        println!("Point Owned By:    {}", dto.point_owned_by);
                        println!("Point Name:        {}", dto.point_name);
                        println!("Job Type:          {}", dto.job_type.to_string());
                        println!("Job Status:        {}", dto.job_status.to_string());
                        println!(
                            "Used Bytes:        {}",
                            FileSize::prettify(dto.used_bytes as u64)
                        );
                        println!(
                            "Total Bytes:       {}",
                            dto.total_bytes
                                .map(|x| FileSize::prettify(x as u64))
                                .unwrap_or("-".into())
                        );
                        if !dto.errors.is_empty() {
                            println!("Errors ({}):\n{}", dto.errors.len(), dto.errors.join("\n"));
                        } else {
                            println!("Errors:            -");
                        }
                        if !dto.messages.is_empty() {
                            println!(
                                "Messages ({}):\n{}",
                                dto.messages.len(),
                                dto.messages.join("\n")
                            );
                        } else {
                            println!("Messages:            -");
                        }
                        println!(
                            "Create Date:       {}",
                            dto.create_date.format("%Y-%m-%d %H:%M:%S")
                        );
                        println!(
                            "End Date:          {}",
                            dto.end_date
                                .map(|x| x.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or("-".to_string())
                        );
                        println!("=================================================\n");
                        println!("(Waiting... Press any key to select options/exit)\n");
                    }
                    Err(e) => {
                        println!("Failed to show job information: {:?}", e);
                        break;
                    }
                }
                last_refresh = Instant::now();
            }

            // Wait up to 100ms for a key event (non-blocking, so loop keeps going)
            if event::poll(Duration::from_millis(100)).unwrap() {
                match event::read().unwrap() {
                    Event::Key(key_event) => {
                        if key_event.is_press() {
                            if key_event.code == KeyCode::Enter {
                                break;
                            }

                            // Go to interactive menu
                            let dto = {
                                let m_api = &*self.api.read().unwrap();
                                if let Some(api) = m_api {
                                    self.rt.block_on(async move {
                                        api.get_all_jobs_for_mount(mount)
                                            .await?
                                            .into_iter()
                                            .find(|x| x.id == j_id)
                                            .ok_or(NeptisError::Str(
                                                "Failed to find the job!".into(),
                                            ))
                                    })
                                } else {
                                    Err(NeptisError::Str("API is not valid!".into()))
                                }
                            };

                            match dto {
                                Ok(dto) => {
                                    if dto.snapshot_id.is_some() {
                                        if Select::new(
                                            "Please select an action",
                                            vec!["Go Back", "View Snapshot"],
                                        )
                                        .prompt_skippable()
                                        .expect("Failed to show prompt!")
                                            == Some("View Snapshot")
                                        {
                                            match {
                                                let m_api = &*self.api.read().unwrap();
                                                if let Some(api) = m_api {
                                                    self.rt.block_on(async move {
                                                        api.get_one_snapshot(
                                                            dto.point_name.as_str(),
                                                            dto.snapshot_id
                                                                .clone()
                                                                .expect(
                                                                    "Expected snapshot to be valid!",
                                                                )
                                                                .as_str(),
                                                        )
                                                            .await
                                                    })
                                                } else {
                                                    Err(NeptisError::Str(
                                                        "API is not valid!".into(),
                                                    ))
                                                }
                                            } {
                                                Ok(s_dto) => {
                                                    self.on_select_snapshot(mount, &[s_dto])
                                                }
                                                Err(_) => break,
                                            }
                                        } else {
                                            break;
                                        }
                                    } else {
                                        if Confirm::new("Do you want to go back")
                                            .with_default(true)
                                            .prompt_skippable()
                                            .map(|x| x.unwrap_or(true))
                                            .expect("Failed to show prompt!")
                                        {
                                            break;
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(p) = point {
            self.on_select_mount(&p);
        } else {
            self.on_view_jobs(mount, None);
        }
    }

    // inspected
    fn on_view_jobs(&self, mount: &str, highlight: Option<&str>) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            // Convert Option<&str> to Option<String> — fully owned
            let mount_owned: String = mount.to_string();
            let highlight_owned: Option<String> = highlight.map(|s| s.to_string());
            if let Some(api) = m_api {
                ModelManager::new(
                    Some(api),
                    vec![ModelProperty::new(
                        "ID",
                        true,
                        |_, _: &mut RepoJobDto| panic!("Not allowed to modify job"),
                        |x| x.id.to_string(),
                    )],
                    Box::new({
                        let mount_owned = mount_owned.clone(); // Already a String now
                        let highlight_owned = highlight_owned.clone();
                        move |ctx| {
                            let api = ctx
                                .api
                                .as_deref()
                                .ok_or(NeptisError::Str("API is not valid!".into()))?;
                            let mount_inner = mount_owned.clone();
                            let highlight_inner = highlight_owned.clone();
                            let sort_jobs = |mut jobs: Vec<RepoJobDto>| {
                                jobs.sort_by(|a, b| match &highlight_inner {
                                    Some(h) => {
                                        let a_is_highlight = a.id.to_string() == *h;
                                        let b_is_highlight = b.id.to_string() == *h;

                                        match (a_is_highlight, b_is_highlight) {
                                            (true, false) => std::cmp::Ordering::Less,
                                            (false, true) => std::cmp::Ordering::Greater,
                                            _ => b.create_date.cmp(&a.create_date),
                                        }
                                    }
                                    None => b.create_date.cmp(&a.create_date),
                                });
                                Ok(jobs)
                            };

                            ctx.rt.block_on(async move {
                                let ret = api.get_all_jobs_for_mount(&mount_inner).await?;
                                sort_jobs(ret)
                            })
                        }
                    }),
                )
                .with_back()
                .do_display()
            } else {
                Err(NeptisError::Str("API is not valid!".into()))
            }
        };

        match ret {
            Ok(x) => match x {
                Some(dto) => self.on_select_job(mount, dto.id, None),
                None => self.on_select_mount(mount),
            },
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(2));
                self.on_select_mount(mount);
            }
        }
    }

    fn on_show_autojob(&self, mount: &str, job: &AutoJobDto) {
        let show_result = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                let result = self.rt.block_on(async {
                    let jobs = api.get_all_jobs(MAX_JOBS, Some(0)).await?;
                    let mut filtered = jobs
                        .into_iter()
                        .filter(|x| x.auto_job.as_deref() == Some(job.task_name.as_str()))
                        .collect::<Vec<_>>();

                    filtered.sort_by(|a, b| b.create_date.cmp(&a.create_date));

                    Ok::<_, NeptisError>(
                        filtered
                            .iter()
                            .map(|x| x.to_short_id_string())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                });

                Some(match result {
                    Ok(text) if text.trim().is_empty() => "None".into(),
                    Ok(text) => text.trim().to_string(),
                    Err(_) => "Failed to load".into(),
                })
            } else {
                None
            }
        };

        if let Some(output) = show_result {
            loop {
                clearscreen::clear().expect("Failed to clear screen!");
                println!("\n============== Jobs:\n{}\n\n", output);
                if Confirm::new("Do you want to go back?")
                    .with_default(true)
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    .unwrap_or(true)
                {
                    break;
                }
            }
        }
        self.on_manage_auto_jobs(mount);
    }

    fn show_rclone_schedules(&self) {
        let server_owned = {
            let _lock = &*self.server.read().unwrap();
            _lock
                .as_ref()
                .expect("Expected a server connection!")
                .server_name
                .clone()
        };
        let smb_name = {
            let m_api = &*self.api.read().unwrap();
            m_api
                .as_ref()
                .map(|x| (format!("{}-smb", x.get_username()), x.get_password()))
        };
        let ret =
            ModelManager::new(
                Some(&(&self.db, &smb_name)),
                vec![
                    ModelProperty::new(
                        "Schedule Name",
                        true,
                        |_, dto: &mut InternalTransferAutoSchedule| match Text::new(
                            "Please enter Schedule Name",
                        )
                        .with_validator(required!())
                        .with_initial_value(&dto.schedule_name)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        {
                            Some(x) => {
                                dto.schedule_name = x;
                                PromptResult::Ok
                            }
                            None => PromptResult::Cancel,
                        },
                        |dto| dto.schedule_name.clone(),
                    ),
                    ModelProperty::new(
                        "Cron Schedule",
                        false,
                        |_, dto: &mut InternalTransferAutoSchedule| match Text::new(
                            "Please enter Cron Schedule (UTC)",
                        )
                        .with_validator(required!())
                        .with_validator(|s: &str| match Schedule::from_str(s) {
                            Ok(_) => Ok(Validation::Valid),
                            Err(_) => Ok(Validation::Invalid("Cron schedule is not valid!".into())),
                        })
                        .with_initial_value(&dto.cron_schedule)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        {
                            Some(x) => {
                                dto.cron_schedule = x;
                                PromptResult::Ok
                            }
                            None => PromptResult::Cancel,
                        },
                        |dto| dto.cron_schedule.clone(),
                    ),
                    ModelProperty::new(
                        "SMB Password",
                        false,
                        |_, dto: &mut InternalTransferAutoSchedule| match Text::new(
                            "Please enter SMB Password",
                        )
                        .with_validator(required!())
                        .with_initial_value(&dto.smb_password)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        {
                            Some(x) => {
                                dto.smb_password = x;
                                PromptResult::Ok
                            }
                            None => PromptResult::Cancel,
                        },
                        |dto| dto.smb_password.clone(),
                    ),
                    ModelProperty::new(
                        "Backup On Finish",
                        false,
                        |_, dto: &mut InternalTransferAutoSchedule| match Confirm::new(
                            "Do you want to Backup on Finish",
                        )
                        .with_default(dto.backup_on_finish)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        {
                            Some(x) => {
                                dto.backup_on_finish = x;
                                PromptResult::Ok
                            }
                            None => PromptResult::Cancel,
                        },
                        |dto| dto.backup_on_finish.to_string(),
                    ),
                ],
                Box::new({
                    let server_owned = server_owned.clone();
                    move |ctx| {
                        let db = ctx
                            .api
                            .as_deref()
                            .ok_or(NeptisError::Str("API is not valid!".into()))?;
                        let server_inner = server_owned.clone(); // clone again for async block
                        Ok(db
                            .0
                            .get_all_transfer_auto_schedules_sync()?
                            .into_iter()
                            .filter(|x| x.server_name == server_inner)
                            .map(|x| x.into())
                            .collect::<Vec<_>>())
                    }
                }),
            )
            .with_select_title("Select a Schedule (Step 1)")
            .with_back()
            .with_delete(Box::new({
                let server_owned = server_owned.clone();
                move |ctx, dto| {
                    let db = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let server_inner = server_owned.clone();
                    Ok(db
                        .0
                        .delete_transfer_auto_schedule_sync(&dto.schedule_name, &server_inner)?)
                }
            }))
            .with_modify(Box::new({
                let server_owned = server_owned.clone();
                move |ctx, all_items, dto| {
                    let db = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let server_inner = server_owned.clone();

                    // Check to see if any saved entries have a different password to
                    // make sure the user has not made a typo or anything.
                    const STR_CHANGE_ALL: &'static str = "*** Change All To This ***";
                    const STR_ACCEPT: &'static str = "*** Accept This Password ***";
                    let mut all_passwords = all_items
                        .iter()
                        .map(|x| x.smb_password.clone())
                        .filter(|x| !x.is_empty())
                        .unique()
                        .collect::<Vec<_>>();

                    let mut final_password = dto.smb_password.clone();
                    if all_passwords.len() > 0 && !all_passwords.contains(&dto.smb_password) {
                        all_passwords.push(STR_CHANGE_ALL.into());
                        all_passwords.push(STR_ACCEPT.into());

                        println!("\n"); // *** create a newline buffer.
                        let ret = Select::new(
                            "Multiple passwords on the same account! Did you make a mistake?",
                            all_passwords,
                        )
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        .unwrap_or(STR_ACCEPT.to_string());
                        if ret == STR_CHANGE_ALL {
                            for item in all_items.iter() {
                                db.0.save_transfer_auto_schedule_sync(&TransferAutoSchedule {
                                    schedule_name: item.schedule_name.clone(),
                                    server_name: server_inner.clone(),
                                    cron_schedule: item.cron_schedule.clone(),
                                    smb_password: dto.smb_password.clone(), // *** not a spelling error
                                    smb_user_name: db.1.clone().map(|x| x.0).ok_or(
                                        NeptisError::Str("Failed to find username!".into()),
                                    )?,
                                    user_password: db.1.clone().map(|x| x.1),
                                    backup_on_finish: dto.backup_on_finish,
                                    last_updated: Utc::now().naive_utc(),
                                })?
                            }
                        } else if ret != STR_ACCEPT {
                            final_password = ret;
                        }
                    }
                    Ok(db
                        .0
                        .save_transfer_auto_schedule_sync(&TransferAutoSchedule {
                            schedule_name: dto.schedule_name.clone(),
                            server_name: server_inner.clone(),
                            cron_schedule: dto.cron_schedule.clone(),
                            smb_password: final_password.clone(),
                            smb_user_name: db
                                .1
                                .clone()
                                .map(|x| x.0)
                                .ok_or(NeptisError::Str("Failed to find username!".into()))?,
                            last_updated: Utc::now().naive_utc(),
                            user_password: db.1.clone().map(|x| x.1),
                            backup_on_finish: dto.backup_on_finish,
                        })?)
                }
            }))
            .do_display();
        match ret {
            Ok(ret) => match ret {
                Some(x) => {
                    const STR_VIEW_JOBS: &'static str = "View Jobs";
                    const STR_MANAGE_ACTIONS: &'static str = "Manage Actions";
                    const STR_START_JOB: &'static str = "Immediate Start";
                    match Select::new(
                        "Please choose an option",
                        vec![STR_VIEW_JOBS, STR_MANAGE_ACTIONS, STR_START_JOB],
                    )
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    {
                        Some(STR_MANAGE_ACTIONS) => {
                            self.on_manage_rclone_actions(&x.schedule_name, &server_owned)
                        }
                        Some(STR_VIEW_JOBS) => {
                            self.on_manage_rclone_jobs(&x.schedule_name, &server_owned)
                        }
                        Some(STR_START_JOB) => {
                            // Attempt to immediately start the job.
                            println!("\n\n*** Attempting to start job. Please wait...");
                            match self.rt.block_on(async {
                                WebApi::ipc_start_auto_job(PostForAutoScheduleStartDto {
                                    server_name: server_owned.clone(),
                                    schedule_name: x.schedule_name.clone(),
                                })
                                .await
                            }) {
                                Ok(_) => {
                                    println!("> Successfully started job!");
                                    thread::sleep(Duration::from_secs(2));
                                    self.on_manage_rclone_jobs(&x.schedule_name, &server_owned);
                                }
                                Err(_) => {
                                    println!("> Failed to start job!");
                                    thread::sleep(Duration::from_secs(2));
                                    self.show_rclone_schedules();
                                }
                            }
                        }
                        _ => self.show_dashboard(),
                    }
                }
                None => self.show_dashboard(),
            },
            Err(e) => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!("*** An unexpected error has occurred. Error: {e}");
                thread::sleep(Duration::from_secs(2));
                self.show_dashboard()
            }
        }
    }

    fn on_manage_rclone_jobs(&self, schedule_name: &str, server_name: &str) {
        clearscreen::clear().expect("Failed to clear screen!");
        if let Ok(jobs) = self.rt.block_on(async {
            WebApi::ipc_get_jobs().await.map(|y| {
                y.into_iter()
                    .filter(|x| x.auto_job_schedule_name == Some(schedule_name.into()))
                    .collect::<Vec<_>>()
            })
        }) {
            const GO_BACK: &'static str = "Go Back";
            let mut choices = jobs
                .iter()
                .map(|x| x.to_short_id_string())
                .collect::<Vec<_>>();
            choices.insert(0, GO_BACK.into());

            if let Some(job_str) = Select::new("Please select a job to manage", choices)
                .prompt_skippable()
                .expect("Failed to show prompt!")
                && let Some(sel_job_id) = jobs
                    .iter()
                    .find(|x| x.to_short_id_string() == job_str)
                    .map(|x| x.job_id)
            {
                self.on_select_rclone_job(sel_job_id, schedule_name, server_name);
            } else {
                self.show_rclone_schedules();
            }
        }
    }

    fn on_manage_rclone_actions(&self, schedule_name: &str, server_name: &str) {
        let schedule_owned = schedule_name.to_string();
        let server_owned = server_name.to_string();
        let fb = FileBrowser::new(NeptisFS::new(self.api.clone(), self.rt.clone()));
        let ret = ModelManager::new(
            Some(&(&self.db, &fb)),
            vec![
                ModelProperty::new(
                    "Action Name",
                    true,
                    |_, dto: &mut InternalTransferAutoJob| match Text::new(
                        "Please enter Action Name",
                    )
                        .with_validator(required!())
                        .with_initial_value(&dto.action_name)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                    {
                        Some(x) => {
                            dto.action_name = x;
                            PromptResult::Ok
                        }
                        None => PromptResult::Cancel,
                    },
                    |dto| dto.action_name.clone(),
                ),
                ModelProperty::new(
                    "Local From",
                    false,
                    |_, dto: &mut InternalTransferAutoJob| match Text::new(
                        "Please enter valid Local From",
                    )
                        .with_validator(required!())
                        .with_validator(|s: &str| {
                            if fs::exists(s).unwrap_or(false) {
                                Ok(Validation::Valid)
                            } else {
                                Ok(Validation::Invalid("Path needs to already exist!".into()))
                            }
                        })
                        .with_initial_value(&dto.local_folder)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                    {
                        Some(x) => {
                            dto.local_folder = x;
                            PromptResult::Ok
                        }
                        None => PromptResult::Cancel,
                    },
                    |dto| dto.local_folder.clone(),
                ),
                ModelProperty::new(
                    "Remote Folder",
                    false,
                    |ctx, dto: &mut InternalTransferAutoJob| {
                        let fs = ctx.api.as_deref().unwrap().1;
                        if Confirm::new("You will now be asked to select a remote SMB folder. Do you want to continue?")
                            .with_default(true)
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            .map(|x| if !x { None } else { Some(x) })
                            .is_none() {
                            return PromptResult::Cancel;
                        }
                        match fs.show_browser(FileBrowserMode::SelectFolderRW) {
                            Some(x) => {
                                dto.smb_folder = x.to_string_lossy().replace("\\", "/");
                                PromptResult::Ok
                            }
                            None => PromptResult::Cancel,
                        }
                    },
                    |dto| dto.smb_folder.clone(),
                ),
                ModelProperty::new(
                    "Enabled",
                    true,
                    |_, dto: &mut InternalTransferAutoJob| match Confirm::new(
                        "Do you want this job enabled",
                    )
                        .with_default(dto.enabled)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                    {
                        Some(x) => {
                            dto.enabled = x;
                            PromptResult::Ok
                        }
                        None => PromptResult::Cancel,
                    },
                    |dto| dto.enabled.to_string(),
                ),
            ],
            Box::new({
                let schedule_owned = schedule_owned.clone();
                let server_owned = server_owned.clone();
                move |ctx| {
                    let db = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let schedule_inner = schedule_owned.clone(); // clone again for async block
                    let server_inner = server_owned.clone();
                    Ok(db
                        .0
                        .get_all_transfer_auto_jobs_sync()?
                        .into_iter()
                        .filter(|x| {
                            x.schedule_name == schedule_inner && x.server_name == server_inner
                        })
                        .map(|x| x.into())
                        .collect::<Vec<_>>())
                }
            }),
        )
            .with_select_title(format!(
                "Select Actions for this Schedule ({}) (Step 2)",
                schedule_name
            ))
            .with_back()
            .with_delete(Box::new({
                let schedule_owned = schedule_owned.clone();
                let server_owned = server_owned.clone();
                move |ctx, dto| {
                    let db = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let schedule_inner = schedule_owned.clone();
                    let server_inner = server_owned.clone();
                    Ok(db.0.delete_transfer_auto_job_sync(
                        &schedule_inner,
                        &server_inner,
                        &dto.action_name,
                    )?)
                }
            }))
            .with_modify(Box::new({
                let schedule_owned = schedule_owned.clone();
                let server_owned = server_owned.clone();
                move |ctx, _, dto| {
                    let db = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let schedule_inner = schedule_owned.clone();
                    let server_inner = server_owned.clone();
                    Ok(db.0.save_transfer_auto_job_sync(&TransferAutoJob {
                        server_name: server_inner,
                        schedule_name: schedule_inner,
                        action_name: dto.action_name.clone(),
                        smb_folder: dto.smb_folder.clone(),
                        local_folder: dto.local_folder.clone(),
                        enabled: dto.enabled,
                    })?)
                }
            }))
            .do_display();

        clearscreen::clear().expect("Failed to clear screen!");
        match ret {
            Ok(x) => {
                if x.is_some() {
                    println!("*** No options are available as of right now.");
                    thread::sleep(Duration::from_secs(2));
                }
            }
            Err(e) => {
                println!("*** An unexpected error has occurred. Error: {e}");
                thread::sleep(Duration::from_secs(2));
            }
        }
        self.show_rclone_schedules();
    }

    // inspected
    fn on_manage_auto_jobs(&self, mount: &str) {
        let ret = {
            let mount_owned = mount.to_string();
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                ModelManager::new(
                    Some(api),
                    vec![
                        ModelProperty::new(
                            "Task Name",
                            true,
                            |_, dto: &mut AutoJobDto| match Text::new("Please enter Task Name")
                                .with_validator(required!())
                                .with_initial_value(dto.task_name.as_str())
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.task_name = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.task_name.clone(),
                        ),
                        ModelProperty::new(
                            "Cron Schedule",
                            false,
                            |_, dto: &mut AutoJobDto| match Text::new("Please enter Cron Schedule")
                                .with_validator(required!())
                                .with_validator(|s: &str| match Schedule::from_str(s) {
                                    Ok(_) => Ok(Validation::Valid),
                                    Err(_) => Ok(Validation::Invalid(
                                        "Cron schedule is not valid!".into(),
                                    )),
                                })
                                .with_initial_value(dto.cron_schedule.as_str())
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.cron_schedule = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.cron_schedule.clone(),
                        ),
                        ModelProperty::new(
                            "Enabled",
                            false,
                            |_, dto: &mut AutoJobDto| match Confirm::new("Do you want it Enabled")
                                .with_default(dto.enabled)
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.enabled = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.enabled.to_string(),
                        ),
                        ModelProperty::new(
                            "Job Type",
                            false,
                            |_, dto: &mut AutoJobDto| match CustomType::<AutoJobType>::new(
                                "Please enter Job Type (Backup/Check)",
                            )
                            .with_starting_input(
                                dto.job_type.to_string().replace("Unknown", "").as_str(),
                            )
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.job_type = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.job_type.to_string(),
                        ),
                    ],
                    Box::new({
                        let mount_owned = mount_owned.clone();
                        move |ctx| {
                            let api = ctx
                                .api
                                .as_deref()
                                .ok_or(NeptisError::Str("API is not valid!".into()))?;
                            let mount_inner = mount_owned.clone(); // clone again for async block
                            ctx.rt.block_on(async move {
                                api.get_all_auto_jobs_for_mount(&mount_inner).await
                            })
                        }
                    }),
                )
                .with_back()
                .with_delete(Box::new({
                    let mount_owned = mount_owned.clone();
                    move |ctx, dto| {
                        let api = ctx
                            .api
                            .as_deref()
                            .ok_or(NeptisError::Str("API is not valid!".into()))?;
                        let mount_inner = mount_owned.clone();
                        ctx.rt.block_on(async move {
                            api.delete_one_auto_job_for_mount(&mount_inner, dto.task_name.as_str())
                                .await
                        })
                    }
                }))
                .with_modify(Box::new(move |ctx, _, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or_else(|| NeptisError::Str("API is not valid!".into()))?;
                    let mount_inner = mount_owned.clone();
                    ctx.rt
                        .block_on(async move {
                            api.put_one_auto_job_for_mount(
                                mount_inner.as_str(),
                                PutForAutoJobWebApi {
                                    task_name: dto.task_name.clone(),
                                    cron_schedule: dto.cron_schedule.clone(),
                                    job_type: dto.job_type.clone(),
                                    enabled: dto.enabled,
                                },
                            )
                            .await
                        })
                        .map(|_| ())
                }))
                .do_display()
            } else {
                Err(NeptisError::Str("API is not valid!".into()))
            }
        };
        match ret {
            Ok(x) => {
                match x {
                    Some(dto) => self.on_show_autojob(mount, &dto), // maybe implement
                    None => self.on_select_mount(mount),
                }
            }
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(2));
                self.on_select_mount(mount);
            }
        }
    }

    fn get_snapshot_mm<'a>(
        api: &'a WebApi,
        mount: &str,
    ) -> ModelManager<'a, SnapshotFileDto, WebApi> {
        let mount_owned = mount.to_string(); // make it owned
        ModelManager::new(
            Some(api),
            vec![ModelProperty::new(
                "ID",
                true,
                |_, _: &mut SnapshotFileDto| panic!("Not allowed to modify snapshot"),
                |x: &SnapshotFileDto| x.id.clone(),
            )],
            Box::new({
                let mount_owned = mount_owned.clone(); // move into the outer closure
                move |ctx| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let mount_inner = mount_owned.clone(); // clone again for async block
                    ctx.rt.block_on(async move {
                        let mut ret = api.get_all_snapshots(&mount_inner).await?;
                        ret.sort_by_key(|x| std::cmp::Reverse(x.time));
                        Ok(ret)
                    })
                }
            }),
        )
        .with_delete(Box::new({
            let mount_owned = mount_owned.clone();
            move |ctx, dto| {
                let api = ctx
                    .api
                    .as_deref()
                    .ok_or(NeptisError::Str("API is not valid!".into()))?;
                let mount_inner = mount_owned.clone();
                ctx.rt.block_on(async move {
                    api.delete_one_snapshot(&mount_inner, dto.id.as_str()).await
                })
            }
        }))
        .with_back()
    }

    fn do_raw_snapshot_select(
        api: &WebApi,
        mount: &str,
    ) -> Result<Option<SnapshotFileDto>, NeptisError> {
        Self::get_snapshot_mm(api, mount).do_display()
    }

    fn do_raw_multi_snapshot_select(
        api: &WebApi,
        mount: &str,
    ) -> Result<Vec<SnapshotFileDto>, NeptisError> {
        Self::get_snapshot_mm(api, mount).do_multi_display()
    }

    // inspected
    fn on_manage_snapshot(&self, mount: &str) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                Self::do_raw_multi_snapshot_select(api, mount)
            } else {
                Err(NeptisError::Str("API is invalid!".into()))
            }
        };
        match ret {
            Ok(x) => {
                if x.is_empty() {
                    self.on_select_mount(mount);
                } else {
                    self.on_select_snapshot(mount, x.as_slice());
                }
            }
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(2));
                self.on_select_mount(mount);
            }
        }
    }

    fn on_start_job(&self, mount: &str, mode: JobType) {
        clearscreen::clear().expect("Failed to clear screen!");
        // If the point is a restore, the user needs to select a snapshot.
        let snap = {
            if mode != JobType::Restore {
                Ok(None)
            } else {
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
                    Self::do_raw_snapshot_select(api, mount)
                } else {
                    Err(NeptisError::Str("API is invalid!".into()))
                }
            }
        };

        clearscreen::clear().expect("Failed to clear screen!");
        if let Ok(s_ret) = snap {
            if s_ret.is_none() && mode == JobType::Restore {
                // The user likely wants to go back - make it happen.
                self.on_select_mount(mount);
                return;
            }

            println!("**** Please confirm the following:");
            println!("Point Name: '{}'", mount);
            println!("Desired Job: {}", mode.to_string().to_uppercase());
            println!("Start Date: Immediate\n");
            if mode == JobType::Restore {
                println!(
                    "Restore From: {}",
                    s_ret.as_ref().unwrap().to_short_id_string()
                )
            }

            let opt = match mode {
                JobType::Backup => Confirm::new("Do you want to lock the snapshot?")
                    .with_default(false)
                    .prompt_skippable()
                    .expect("Failed to show prompt!"),
                // JobType::Restore => Confirm::new("Do you want to overwrite data?")
                //     .prompt_skippable()
                //     .expect("Failed to show prompt!"),  REMOVED 7-7-25
                _ => Some(false),
            };

            if let Some(opt) = opt {
                if Confirm::new("Do you want to proceed?")
                    .with_default(true)
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    .unwrap_or(false)
                {
                    // Initialize the job and attempt to show it off.
                    println!("Creating job...");
                    let ret = {
                        let m_api = &*self.api.read().unwrap();
                        if let Some(api) = m_api {
                            self.rt.block_on(async {
                                match mode {
                                    JobType::Backup => api.post_one_backup(mount, opt).await,
                                    JobType::Check => api.post_one_check(mount).await,
                                    JobType::Restore => {
                                        api.post_one_restore(mount, s_ret.unwrap().id.as_str())
                                            .await
                                    }
                                    _ => Err(NeptisError::Str("Invalid option selected".into())),
                                }
                            })
                        } else {
                            Err(NeptisError::Str("API is not valid!".into()))
                        }
                    };
                    match ret {
                        Ok(x) => {
                            println!("**** Job created successfully! ID: {}", x.id);
                            thread::sleep(Duration::from_secs(2));
                            self.on_select_job(mount, x.id.clone(), Some(mount.to_string()));
                        }
                        Err(e) => {
                            println!(
                                "**** An unexpected error has occurred while changing the password. ****\n{}",
                                e.to_string()
                            );
                            thread::sleep(Duration::from_secs(2));
                            self.on_select_mount(mount);
                        }
                    }
                } else {
                    self.on_select_mount(mount);
                }
            } else {
                self.on_select_mount(mount);
            }
        } else {
            println!("**** An unexpected error has occurred. Going back in 2 seconds...");
            thread::sleep(Duration::from_secs(2));
            self.on_select_mount(mount);
        }
    }

    // inspected
    fn on_select_mount(&self, mount: &str) {
        clearscreen::clear().expect("Failed to clear screen!");
        println!("**** Currently selected Mount: {}", mount);
        // Attempt to pull the real up-to-date information on the mount.
        match {
            let mut stats = None;
            let mut smb = None;
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                stats = self.rt.block_on(async {
                    api.get_all_mounts()
                        .await
                        .ok()
                        .map(|x| x.into_iter().find(|y| y.name == mount))
                        .flatten()
                });
                smb = self.rt.block_on(async {
                    api.get_one_user(&api.get_username())
                        .await
                        .ok()
                        .map(|x| x.is_smb)
                })
            }
            (stats, smb.unwrap_or(false))
        } {
            (Some(stats), smb) => {
                let d_total = stats.usage.b_data_total;
                let d_used = stats.usage.b_data_used;
                let d_free = d_total - d_used;

                let r_total = stats.usage.b_repo_total;
                let r_used = stats.usage.b_repo_used;
                let r_free = r_total - r_used;

                fn prettify_bytes(
                    b_total: i64,
                    b_used: Option<i64>,
                    b_free: Option<i64>,
                ) -> String {
                    format!(
                        "{} / {} ({} remaining)",
                        b_used
                            .map(|x| FileSize::prettify(x as u64))
                            .unwrap_or("N/A".into()),
                        FileSize::prettify(b_total as u64),
                        b_free
                            .map(|x| FileSize::prettify(x as u64))
                            .unwrap_or("unknown space".into())
                    )
                }
                if smb {
                    println!(
                        "**** SMB Enabled at: \\\\IP\\{}-{}-<data or repo>\n",
                        stats.owned_by.as_str(),
                        stats.name.as_str()
                    );
                }
                println!(
                    "Created At: {}",
                    stats
                        .date_created
                        .and_utc()
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %I:%M:%S %p")
                        .to_string()
                );
                println!(
                    "Data Accessed At: {}",
                    stats
                        .data_accessed
                        .and_utc()
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %I:%M:%S %p")
                        .to_string()
                );
                println!(
                    "Repo Accessed At: {}",
                    stats
                        .repo_accessed
                        .and_utc()
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %I:%M:%S %p")
                        .to_string()
                );
                println!();
                println!(
                    "Data Usage: {}",
                    prettify_bytes(d_total as i64, Some(d_used as i64), Some(d_free as i64))
                );
                println!(
                    "Repo Usage: {}",
                    prettify_bytes(r_total as i64, Some(r_used as i64), Some(r_free as i64))
                );
            }
            _ => println!("> FAILED to display additional statistics"),
        }

        // Ask the user for the information (Snapshot, Jobs, Auto Jobs)
        const STR_MANAGE_SNAPSHOT: &'static str = "Manage Snapshots";
        const STR_MANAGE_JOB: &'static str = "View Jobs";
        const STR_MANAGE_AUTO_JOB: &'static str = "Manage Auto-Jobs";
        const STR_START_BACKUP: &'static str = "Start Backup";
        const STR_START_CHECK: &'static str = "Start Check";
        const STR_START_RESTORE: &'static str = "Start Restore";
        const STR_GO_BACK: &'static str = "Go Back";

        match Select::new(
            "Please select your desired action",
            vec![
                STR_GO_BACK,
                STR_MANAGE_SNAPSHOT,
                STR_MANAGE_JOB,
                STR_MANAGE_AUTO_JOB,
                STR_START_BACKUP,
                STR_START_CHECK,
                STR_START_RESTORE,
            ],
        )
        .prompt_skippable()
        .expect("Failed to show prompt!")
        .unwrap_or(STR_GO_BACK)
        {
            STR_MANAGE_SNAPSHOT => self.on_manage_snapshot(mount),
            STR_MANAGE_JOB => self.on_view_jobs(mount, None),
            STR_MANAGE_AUTO_JOB => self.on_manage_auto_jobs(mount),
            STR_START_BACKUP => self.on_start_job(mount, JobType::Backup),
            STR_START_CHECK => self.on_start_job(mount, JobType::Check),
            STR_START_RESTORE => self.on_start_job(mount, JobType::Restore),
            _ => self.show_points(), // go back
        }
    }

    // inspected
    fn on_select_user(&self, user: &UserDto, ack: bool) {
        clearscreen::clear().expect("Failed to clear screen!");

        if ack
            || Confirm::new(
                "The only available option is to change the password. Do you want to do this?",
            )
            .prompt_skippable()
            .expect("Failed to show prompt!")
            .unwrap_or(false)
        {
            let a_str = format!("Please enter password for {}", user.user_name.as_str());
            if let Some(p) = Password::new(a_str.as_str())
                .with_validator(required!())
                .prompt_skippable()
                .expect("Failed to show prompt!")
            {
                match (|| {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        let p2 = p.clone();
                        self.rt
                            .block_on(async move {
                                api.put_one_user(
                                    user.user_name.as_str(),
                                    UserForUpdateApi {
                                        first_name: None,
                                        last_name: None,
                                        is_admin: None,
                                        max_data_bytes: None,
                                        max_snapshot_bytes: None,
                                        password: Some(p2),
                                    },
                                )
                                .await
                            })
                            .map(|_| ())?;
                        Self::_on_change_password(&self.db, &user.user_name, &p)?;
                        Ok(p)
                    } else {
                        Err(NeptisError::Str("API is invalid!".into()))
                    }
                })() {
                    Ok(_pass) => {
                        println!("**** Password changed successfully.");
                        thread::sleep(Duration::from_secs(2));
                        self.show_users();
                    }
                    Err(e) => {
                        println!(
                            "**** An unexpected error has occurred while changing the password. ****\n{}",
                            e.to_string()
                        );
                        thread::sleep(Duration::from_secs(2));
                        self.show_users();
                    }
                }
            }
        }
        self.show_users();
    }

    fn get_luser_stats(&self, api: &WebApi, is_breakdown: bool) -> (String, bool) {
        if let Ok(user) = {
            self.rt
                .block_on(async { api.get_one_user(api.get_username().as_str()).await })
        } {
            // The user pulled successfully - attempt to get the maximum bytes.
            let usage_str = self
                .rt
                .block_on(async {
                    api.get_all_mounts().await.map(|mounts| {
                        if is_breakdown {
                            // Create breakdown by each point
                            let mut data_points: Vec<(String, u64, u64)> = mounts.iter()
                                .map(|y| (
                                    y.name.clone(),
                                    y.usage.b_data_used as u64,
                                    y.usage.b_data_total as u64
                                ))
                                .collect();
                            let mut repo_points: Vec<(String, u64, u64)> = mounts.iter()
                                .map(|y| (
                                    y.name.clone(),
                                    y.usage.b_repo_used as u64,
                                    y.usage.b_repo_total as u64
                                ))
                                .collect();
                            // Sort by total (used bytes) descending
                            data_points.sort_by(|a, b| b.1.cmp(&a.1));
                            repo_points.sort_by(|a, b| b.1.cmp(&a.1));
                            let data_total = mounts.iter().map(|y| y.usage.b_data_total as i64).sum::<i64>() as u64;
                            let repo_total = mounts.iter().map(|y| y.usage.b_repo_total as i64).sum::<i64>() as u64;
                            let data_breakdown = data_points.iter()
                                .map(|(name, used, max)| {
                                    format!("  • {}: {} / {}", name, FileSize::prettify(*used), FileSize::prettify(*max))
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            let repo_breakdown = repo_points.iter()
                                .map(|(name, used, max)| {
                                    format!("  • {}: {} / {}", name, FileSize::prettify(*used), FileSize::prettify(*max))
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            format!(
                                "Data Point Allocation: {} / {}\n{}\n\nRepo Point Allocation: {} / {}\n{}",
                                FileSize::prettify(data_total),
                                FileSize::prettify(user.max_data_bytes as u64),
                                data_breakdown,
                                FileSize::prettify(repo_total),
                                FileSize::prettify(user.max_repo_bytes as u64),
                                repo_breakdown
                            )
                        } else {
                            // Original behavior
                            let (d_max, r_max, d_used, r_used) = (
                                FileSize::prettify(mounts.iter().map(|y| y.usage.b_data_total as i64).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.usage.b_repo_total as i64).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.usage.b_data_used as i64).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.usage.b_repo_used as i64).sum::<i64>() as u64),
                            );
                            format!(
                                "Data Point Allocation: {d_max} / {}\nData Point File Usage: {d_used} / {d_max}\n\nRepo Point Allocation: {r_max} / {}\nRepo Point File Usage: {r_used} / {r_max}",
                                FileSize::prettify(user.max_data_bytes as u64),
                                FileSize::prettify(user.max_repo_bytes as u64)
                            )
                        }
                    })
                })
                .unwrap_or("Failed to calculate Data Total File Usage".into());
            (
                format!(
                    "Logged in as {}\nPrivileged: {}\n{}",
                    user.user_name.as_str(),
                    if user.is_admin { "Yes" } else { "No" },
                    usage_str
                ),
                user.is_admin,
            )
        } else {
            ("Failed to calculate User Information".into(), false)
        }
    }

    //noinspection ALL
    async fn _ensure_job_good(api: &WebApi, id: Uuid) -> Result<(), NeptisError> {
        println!(
            "**** Sent request. Server responded with Job #{:.6}...",
            &id.to_string()
        );
        thread::sleep(Duration::from_secs(2));
        for i in 0..20 {
            let job = api.get_one_job(id).await?;
            if job.job_status == JobStatus::Successful {
                println!("> Operation successful!");
                thread::sleep(Duration::from_secs(1));
                return Ok(());
            } else if job.job_status == JobStatus::Failed {
                println!("> Operation failed. Error(s):\n{}", job.errors.join("\n"));
                thread::sleep(Duration::from_secs(2));
                return Err(NeptisError::Str("Operation failed".into()));
            } else {
                println!("> Waiting for job to finish... ({i}/20) tries");
            }
            thread::sleep(Duration::from_secs(2));
        }
        println!("> Operation timed out without response.");
        thread::sleep(Duration::from_secs(1));
        Err(NeptisError::Str("Operation failed".into()))
    }

    fn show_points(&self) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                // Attempt to pull the maximum user statistics
                let stats = self.get_luser_stats(api, false);
                println!("{}\n", stats.0.as_str());
                ModelManager::new(
                    Some(api),
                    vec![
                        ModelProperty::new(
                            "Name",
                            true,
                            |_, dto: &mut InternalMountDto| match Text::new(
                                "Please enter Mount Name",
                            )
                            .with_initial_value(dto.name.as_str())
                            .with_validator(required!())
                            .with_validator(|m_name: &str| {
                                if !regex::Regex::new(r"^[a-z_][a-z0-9_-]*$")
                                    .expect("Expected regex to work")
                                    .is_match(m_name)
                                {
                                    Ok(Validation::Invalid("Bad name!".into()))
                                } else {
                                    Ok(Validation::Valid)
                                }
                            })
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.name = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.name.clone(),
                        ),
                        ModelProperty::new(
                            "Data Bytes",
                            false,
                            |ctx, dto: &mut InternalMountDto| {
                                // Attempt to pull the data interval as set by the server.
                                match if let Some(api) = ctx.api.as_deref() {
                                    ctx.rt.block_on(async move {
                                        Ok((
                                            api.get_info().await?.data_info.b_blk_size,
                                            api.get_one_user(&api.get_username())
                                                .await?
                                                .free_data_bytes,
                                        ))
                                    })
                                } else {
                                    Err(NeptisError::Str(
                                        "Failed to pull API. Are you connected?".into(),
                                    ))
                                } {
                                    Ok((b_int, _b_max)) => {
                                        let si =
                                            FileSize::from_bytes(dto.data_bytes as u64).to_string();
                                        match CustomType::<FileSize>::new(
                                            "Please enter maximum data size",
                                        )
                                        .with_starting_input(si.as_str())
                                        .with_validator(move |input: &FileSize| {
                                            if input.get_bytes() < 100000 {
                                                Ok(Validation::Invalid(
                                                    "You must enter at least 100K bytes!".into(),
                                                ))
                                            } else if input.get_bytes() % b_int != 0 {
                                                Ok(Validation::Invalid(
                                                    format!(
                                                        "Your input must be in intervals of {}",
                                                        FileSize::prettify(b_int)
                                                    )
                                                    .into(),
                                                ))
                                            } else {
                                                Ok(Validation::Valid)
                                            }
                                        })
                                        .prompt_skippable()
                                        .expect("Failed to show prompt!")
                                        .map(|x| x.get_bytes() as i64)
                                        {
                                            Some(x) => {
                                                dto.data_bytes = x;
                                                PromptResult::Ok
                                            }
                                            None => PromptResult::Cancel,
                                        }
                                    }
                                    _ => PromptResult::Cancel,
                                }
                            },
                            |x| FileSize::from_bytes(x.data_bytes as u64).to_string(),
                        ),
                        ModelProperty::new(
                            "Repo Bytes",
                            false,
                            |ctx, dto: &mut InternalMountDto| {
                                // Attempt to pull the data interval as set by the server.
                                match if let Some(api) = ctx.api.as_deref() {
                                    ctx.rt.block_on(async move {
                                        Ok((
                                            api.get_info().await?.repo_info.b_blk_size,
                                            api.get_one_user(&api.get_username())
                                                .await?
                                                .free_repo_bytes,
                                        ))
                                    })
                                } else {
                                    Err(NeptisError::Str(
                                        "Failed to pull API. Are you connected?".into(),
                                    ))
                                } {
                                    Ok((b_int, _b_max)) => {
                                        let si =
                                            FileSize::from_bytes(dto.data_bytes as u64).to_string();
                                        match CustomType::<FileSize>::new(
                                            "Please enter maximum repo size",
                                        )
                                        .with_starting_input(si.as_str())
                                        .with_validator(move |input: &FileSize| {
                                            if input.get_bytes() < 100000 {
                                                Ok(Validation::Invalid(
                                                    "You must enter at least 100K bytes!".into(),
                                                ))
                                            } else if input.get_bytes() % b_int != 0 {
                                                Ok(Validation::Invalid(
                                                    format!(
                                                        "Your input must be in intervals of {}",
                                                        FileSize::prettify(b_int)
                                                    )
                                                    .into(),
                                                ))
                                            } else {
                                                Ok(Validation::Valid)
                                            }
                                        })
                                        .prompt_skippable()
                                        .expect("Failed to show prompt!")
                                        .map(|x| x.get_bytes() as i64)
                                        {
                                            Some(x) => {
                                                dto.repo_bytes = x;
                                                PromptResult::Ok
                                            }
                                            None => PromptResult::Cancel,
                                        }
                                    }
                                    _ => PromptResult::Cancel,
                                }
                            },
                            |x| FileSize::from_bytes(x.repo_bytes as u64).to_string(),
                        ),
                    ],
                    Box::new(|ctx| {
                        let api = ctx
                            .api
                            .as_deref()
                            .ok_or(NeptisError::Str("API is not valid!".into()))?;
                        ctx.rt.block_on(async move {
                            api.get_all_mounts().await.map(|y| {
                                y.into_iter()
                                    .map(|x| InternalMountDto {
                                        name: x.name.clone(),
                                        data_bytes: x.usage.b_data_total as i64,
                                        repo_bytes: x.usage.b_repo_total as i64,
                                    })
                                    .collect::<Vec<_>>()
                            })
                        })
                    }),
                )
                .with_create_title(stats.0.clone())
                .with_modify_title(stats.0.clone())
                .with_back()
                .with_delete(Box::new(|ctx, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt.block_on(async move {
                        let ret = api.delete_one_mount(dto.name.as_str()).await?;
                        Self::_ensure_job_good(api, ret.id).await
                    })
                }))
                .with_modify(Box::new(|ctx, _, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt.block_on(async move {
                        let ret = api
                            .put_one_mount(
                                dto.name.as_str(),
                                PutForMountApi {
                                    data_bytes: dto.data_bytes,
                                    repo_bytes: dto.repo_bytes,
                                },
                            )
                            .await?;
                        Self::_ensure_job_good(api, ret.id).await
                    })
                }))
                .do_display()
            } else {
                Err(NeptisError::Str("API is not valid!".into()))
            }
        };
        match ret {
            Ok(x) => {
                match x {
                    Some(dto) => self.on_select_mount(dto.name.as_str()),
                    None => self.show_dashboard(), // assume user clicked back
                }
            }
            Err(e) => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!(
                    "**** An unexpected error has occurred while modifying users. Clearing in 5 secs... ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(5));
                self.begin();
            }
        }
    }

    fn show_notifications(&self) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                ModelManager::new(
                    Some(api),
                    vec![
                        ModelProperty::new(
                            "Alert Mode",
                            false,
                            |_, dto: &mut SubscriptionDto| match CustomType::<AlertMode>::new(
                                "Please enter the Mode (Discord/Email/Post)",
                            )
                            .with_starting_input(&dto.mode.to_string())
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.mode = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.mode.to_string(),
                        ),
                        ModelProperty::new(
                            "Endpoint",
                            false,
                            |_, dto: &mut SubscriptionDto| match Text::new(
                                "Please enter the endpoint",
                            )
                            .with_initial_value(&dto.endpoint.to_string())
                            .with_validator(required!())
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.endpoint = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.endpoint.clone(),
                        ),
                        ModelProperty::new(
                            "Triggers",
                            false,
                            |_, dto: &mut SubscriptionDto| {
                                let opts = vec![
                                    AlertTrigger::UserMessage,
                                    AlertTrigger::PointCreated,
                                    AlertTrigger::PointModified,
                                    AlertTrigger::PointDeleted,
                                    AlertTrigger::JobStarted,
                                    AlertTrigger::JobFinished,
                                    AlertTrigger::JobError,
                                    AlertTrigger::ServerShutdown,
                                    AlertTrigger::ServerRestart,
                                    AlertTrigger::AutoJobCreated,
                                    AlertTrigger::AutoJobModified,
                                    AlertTrigger::AutoJobDeleted,
                                    AlertTrigger::SnapshotLocked,
                                    AlertTrigger::SnapshotUnlocked,
                                    AlertTrigger::SnapshotDeleted,
                                ];
                                match MultiSelect::new("Please select all triggers", opts.clone())
                                    .with_validator(|choices: &[ListOption<&AlertTrigger>]| {
                                        if choices.is_empty() {
                                            Ok(Validation::Invalid(
                                                "Please select at least one trigger.".into(),
                                            ))
                                        } else {
                                            Ok(Validation::Valid)
                                        }
                                    })
                                    .with_default(
                                        &opts
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(i, val)| {
                                                if dto.triggers.contains(val) {
                                                    Some(i)
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect::<Vec<_>>(),
                                    )
                                    .prompt_skippable()
                                    .expect("Failed to show prompt!")
                                {
                                    Some(x) => {
                                        dto.triggers = x;
                                        PromptResult::Ok
                                    }
                                    None => PromptResult::Cancel,
                                }
                            },
                            |x| {
                                x.triggers
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(",")
                            },
                        ),
                        ModelProperty::new(
                            "Enabled",
                            false,
                            |_, dto: &mut SubscriptionDto| match Confirm::new(
                                "Do you want it enabled",
                            )
                            .with_default(dto.enabled)
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    dto.enabled = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.enabled.to_string(),
                        ),
                    ],
                    Box::new(|ctx| {
                        let api = ctx
                            .api
                            .as_deref()
                            .ok_or(NeptisError::Str("API is not valid!".into()))?;
                        ctx.rt
                            .block_on(async move { api.get_all_subscriptions().await })
                    }),
                )
                .with_delete(Box::new(|ctx, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt.block_on(async move {
                        api.delete_one_subscription(&dto.id.to_string()).await
                    })
                }))
                .with_modify(Box::new(|ctx, current_subs, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    let is_creating = !current_subs.iter().any(|x| x.id == dto.id);
                    ctx.rt.block_on(async move {
                        if is_creating {
                            api.post_one_subscription(PostForSubscriptionApi {
                                mode: dto.mode.clone(),
                                enabled: dto.enabled,
                                endpoint: dto.endpoint.clone(),
                                triggers: dto.triggers.clone(),
                            })
                            .await
                        } else {
                            api.put_one_subscription(
                                &dto.id.to_string(),
                                PutForSubscriptionApi {
                                    mode: Some(dto.mode.clone()),
                                    endpoint: Some(dto.endpoint.clone()),
                                    triggers: Some(dto.triggers.clone()),
                                    enabled: Some(dto.enabled),
                                },
                            )
                            .await
                            .map(|_| ())
                        }
                        .map(|_| ())
                    })
                }))
                .with_back()
                .do_display()
            } else {
                Err(NeptisError::Str("API is invalid!".into()))
            }
        };
        match ret {
            Err(e) => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!(
                    "**** An unexpected error has occurred. Clearing in 5 secs: {}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(5));
                self.begin();
            }
            _ => self.show_dashboard(),
        }
    }

    // inspected
    fn show_users(&self) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                ModelManager::new(
                    Some(api),
                    vec![
                        ModelProperty::new(
                            "Username",
                            true,
                            |_, user: &mut UserDto| match Text::new("Please enter Username")
                                .with_initial_value(user.user_name.as_str())
                                .with_validator(required!())
                                .with_validator(|m_name: &str| {
                                    if !regex::Regex::new(r"^[a-z_][a-z0-9_-]*$")
                                        .expect("Expected regex to work")
                                        .is_match(m_name)
                                    {
                                        Ok(Validation::Invalid("Bad name!".into()))
                                    } else {
                                        Ok(Validation::Valid)
                                    }
                                })
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    user.user_name = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.user_name.clone(),
                        ),
                        ModelProperty::new(
                            "First Name",
                            false,
                            |_, user: &mut UserDto| match Text::new("Please enter First Name")
                                .with_initial_value(user.first_name.as_str())
                                .with_validator(required!())
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    user.first_name = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.first_name.clone(),
                        ),
                        ModelProperty::new(
                            "Last Name",
                            false,
                            |_, user: &mut UserDto| match Text::new("Please enter Last Name")
                                .with_initial_value(user.last_name.as_str())
                                .with_validator(required!())
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    user.last_name = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.last_name.clone(),
                        ),
                        ModelProperty::new(
                            "Is Admin",
                            false,
                            |_, user: &mut UserDto| match Confirm::new("Should the user be admin")
                                .with_default(user.is_admin)
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                            {
                                Some(x) => {
                                    user.is_admin = x;
                                    PromptResult::Ok
                                }
                                None => PromptResult::Cancel,
                            },
                            |x| x.is_admin.to_string(),
                        ),
                        ModelProperty::new(
                            "Max Data",
                            false,
                            |_, user: &mut UserDto| {
                                let si = FileSize::from(user.max_data_bytes as u64).to_string();
                                match CustomType::<FileSize>::new("Please enter maximum data size")
                                    .with_starting_input(si.as_str())
                                    .with_validator(|input: &FileSize| {
                                        if input.get_bytes() < 10000 {
                                            Ok(Validation::Invalid(
                                                "You must enter at least 10K bytes!".into(),
                                            ))
                                        } else {
                                            Ok(Validation::Valid)
                                        }
                                    })
                                    .prompt_skippable()
                                    .expect("Failed to show prompt!")
                                    .map(|x| x.get_bytes() as usize)
                                {
                                    Some(x) => {
                                        user.max_data_bytes = x;
                                        PromptResult::Ok
                                    }
                                    None => PromptResult::Cancel,
                                }
                            },
                            |x| FileSize::prettify(x.max_data_bytes as u64),
                        ),
                        ModelProperty::new(
                            "Max Repo",
                            false,
                            |_, user: &mut UserDto| {
                                let si = FileSize::from(user.max_repo_bytes as u64).to_string();
                                match CustomType::<FileSize>::new("Please enter maximum repo size")
                                    .with_starting_input(si.as_str())
                                    .with_validator(|input: &FileSize| {
                                        if input.get_bytes() < 10000 {
                                            Ok(Validation::Invalid(
                                                "You must enter at least 10K bytes!".into(),
                                            ))
                                        } else {
                                            Ok(Validation::Valid)
                                        }
                                    })
                                    .prompt_skippable()
                                    .expect("Failed to show prompt!")
                                    .map(|x| x.get_bytes() as usize)
                                {
                                    Some(x) => {
                                        user.max_repo_bytes = x;
                                        PromptResult::Ok
                                    }
                                    None => PromptResult::Cancel,
                                }
                            },
                            |x| FileSize::prettify(x.max_repo_bytes as u64),
                        ),
                    ],
                    Box::new(|ctx| {
                        let api = ctx
                            .api
                            .as_deref()
                            .ok_or(NeptisError::Str("API is not valid!".into()))?;
                        ctx.rt.block_on(async move { api.get_all_users().await })
                    }),
                )
                .with_back()
                .with_delete(Box::new(|ctx, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt
                        .block_on(async move { api.delete_one_user(dto.user_name.as_str()).await })
                }))
                .with_create_title(format!("The default password will be {}", DEFAULT_PASS))
                .with_modify(Box::new(|ctx, current_users, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    // We need to see if we are CREATING or UPDATING a specific user.
                    let is_creating = !current_users
                        .iter()
                        .any(|x| x.user_name == dto.user_name.as_str());
                    ctx.rt.block_on(async move {
                        if is_creating {
                            api.create_one_user(UserForCreateApi {
                                user_name: dto.user_name.clone(),
                                password: DEFAULT_PASS.into(),
                                first_name: dto.first_name.clone(),
                                last_name: dto.last_name.clone(),
                                is_admin: dto.is_admin,
                                max_data_bytes: dto.max_data_bytes as i64,
                                max_snapshot_bytes: dto.max_repo_bytes as i64,
                            })
                            .await
                        } else {
                            api.put_one_user(
                                dto.user_name.as_str(),
                                UserForUpdateApi {
                                    first_name: Some(dto.first_name.clone()),
                                    last_name: Some(dto.last_name.clone()),
                                    is_admin: Some(dto.is_admin),
                                    max_data_bytes: Some(dto.max_data_bytes as i64),
                                    max_snapshot_bytes: Some(dto.max_repo_bytes as i64),
                                    password: None, // password will be set separately
                                },
                            )
                            .await
                        }
                        .map(|_| ())
                    })
                }))
                .do_display()
            } else {
                Err(NeptisError::Str("API is invalid!".into()))
            }
        };
        match ret {
            Ok(x) => match x {
                Some(dto) => self.on_select_user(&dto, false),
                None => self.show_dashboard(),
            },
            Err(e) => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!(
                    "**** An unexpected error has occurred. Clearing in 5 secs: {}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(5));
                self.begin();
            }
        }
    }

    //inspected
    fn show_system(&self) {
        use crossterm::{
            event::{self, Event},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::{
            io::Write,
            process, thread,
            time::{Duration, Instant},
        };
        const STR_REFRESH: &str = "Cancel";
        const STR_SHUTDOWN: &str = "Shutdown";
        const STR_RESTART: &str = "Restart";
        const STR_BACK: &str = "Go Back";
        let mut last_refresh = Instant::now();
        let mut first_time: bool = true;
        let mut is_safe: bool = false;
        loop {
            if first_time || last_refresh.elapsed().as_secs() >= 10 {
                first_time = false;
                // First: lock and get info string
                clearscreen::clear().expect("Failed to clear screen!");
                let ret = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        if let Ok(info) = self.rt.block_on(async { api.get_info().await }) {
                            info.print_info();
                            Ok(())
                        } else {
                            Err(NeptisError::Str("Failed to pull info!".into()))
                        }
                    } else {
                        Err(NeptisError::Str("API is not valid!".into()))
                    }
                };
                if ret.is_err() {
                    println!("**** Failed to show system information!");
                }
                is_safe = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        self.rt
                            .block_on(async { api.can_kill_safe().await.unwrap_or(false) })
                    } else {
                        self.show_dashboard();
                        return;
                    }
                };

                if !is_safe {
                    println!(
                        "***** WARNING: The server has indicated that Shutdown / Restart is unsafe!\nThis can be due to several factors: including running jobs,\nSMB restarts, or others.",
                    );
                }
                println!("\n\nPress <ENTER> to show options...");
                last_refresh = Instant::now();
            }

            // Poll for a keypress non-blocking
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(k) = event::read().unwrap()
                    && k.is_press()
                {
                    break;
                }
            }
        }

        fn handle_result<F>(result: Result<(), NeptisError>, callback: F)
        where
            F: FnOnce(),
        {
            match result {
                Ok(_) => {
                    println!("Successfully performed the operation!");
                    thread::sleep(Duration::from_secs(3));
                    callback();
                }
                Err(_) => {
                    println!("Operation failed!");
                    callback();
                }
            }
        }

        let choice = Select::new(
            "Please select an option",
            vec![STR_REFRESH, STR_BACK, STR_SHUTDOWN, STR_RESTART],
        )
        .prompt_skippable()
        .expect("Failed to show prompt!")
        .unwrap_or(STR_BACK);

        fn handle_unsafe(is_safe: bool) -> bool {
            if is_safe {
                return true;
            }
            clearscreen::clear().expect("Failed to clear screen!");
            println!("**** DANGER: Powering down the system may corrupt jobs or services!");
            println!("\n");
            Confirm::new("Please confirm your decision")
                .prompt_skippable()
                .expect("Failed to show prompt!")
                .unwrap_or(false)
        }

        match choice {
            STR_REFRESH => self.show_system(),
            STR_SHUTDOWN => {
                if !handle_unsafe(is_safe) {
                    self.show_dashboard();
                } else {
                    let result = {
                        let m_api = &*self.api.read().unwrap();
                        if let Some(api) = m_api {
                            self.rt.block_on(async { api.shutdown().await })
                        } else {
                            Err(NeptisError::Str("API is invalid!".into()))
                        }
                    };
                    handle_result(result, || self.show_dashboard());
                }
            }

            STR_RESTART => {
                if !handle_unsafe(is_safe) {
                    self.show_dashboard();
                } else {
                    let result = {
                        let m_api = &*self.api.read().unwrap();
                        if let Some(api) = m_api {
                            self.rt.block_on(async { api.restart().await })
                        } else {
                            Err(NeptisError::Str("API is invalid!".into()))
                        }
                    };
                    handle_result(result, || self.show_dashboard());
                }
            }

            _ => self.show_dashboard(),
        }
    }

    fn _on_change_password(
        db: &DbController,
        user_name: &str,
        new_password: &str,
    ) -> Result<(), NeptisError> {
        // Attempt to pull the password information.
        let smb_name = format!("{}-smb", user_name);
        let all_jobs = db.get_all_transfer_auto_schedules_sync()?;
        for mut job in all_jobs {
            if job.smb_user_name == smb_name {
                job.user_password = Some(new_password.into());
            }
            db.save_transfer_auto_schedule_sync(&job)?;
        }
        Ok(())
    }

    // inspected
    fn show_change_password(&self) {
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                if let Some(p) = Password::new("Please enter your new password")
                    .with_validator(required!())
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                {
                    match self
                        .rt
                        .block_on(async { api.put_password(&p).await })
                        .and_then(|_| Self::_on_change_password(&self.db, &api.get_username(), &p))
                    {
                        Ok(_) => {
                            println!("**** Successfully changed password!")
                        }
                        Err(_) => println!("**** Failed to change password!"),
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        self.show_dashboard();
    }

    fn show_point_breakdown(&self) {
        loop {
            clearscreen::clear().expect("Failed to clear screen!");
            {
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
                    println!("Connection: {}\n", api.to_string());
                    println!("{}", self.get_luser_stats(api, true).0);
                } else {
                    println!("Failed to load additional statistics.");
                }
            }
            println!("\n\n\n");
            if Confirm::new("Do you want to go back")
                .with_default(true)
                .prompt_skippable()
                .expect("Failed to show prompt!")
                .unwrap_or(true)
            {
                break;
            }
        }
        self.show_dashboard();
    }

    fn show_smb(&self) {
        let handle_sync = |user_name: &str, new_pass: &str| -> Result<(), NeptisError> {
            // 6-24-25: See if the user wants to change their client-side jobs (if any).
            if let Some(server_name) = {
                let _lock = &*self.server.read().unwrap();
                _lock.as_ref().map(|x| x.server_name.clone())
            } {
                use cron_descriptor::cronparser::cron_expression_descriptor::get_description_cron;
                let sync_jobs = self
                    .db
                    .get_all_transfer_auto_schedules_sync()?
                    .into_iter()
                    .filter(|x| {
                        x.server_name == server_name
                            && x.smb_user_name == format!("{}-smb", user_name)
                    })
                    .collect::<Vec<_>>();
                let job_str = sync_jobs
                    .iter()
                    .map(|x| {
                        format!(
                            "{} ({})",
                            x.schedule_name,
                            get_description_cron(&x.cron_schedule).unwrap_or("N/A".into())
                        )
                    })
                    .join("\n");
                if sync_jobs.len() > 0 {
                    println!("\n{job_str}\n");
                    if Confirm::new(
                        "You have several sync jobs. Do you want to set the new password to the above items?",
                    ).prompt_skippable()
                        .expect("Failed to show prompt!")
                        .unwrap_or(false) {
                        // If we are setting the password - update it for all!
                        for mut item in sync_jobs {
                            item.smb_password = new_pass.to_string();
                            self.db.save_transfer_auto_schedule_sync(&item)?;
                        }
                        println!("*** Done!");
                    }
                }
            }

            Ok(())
        };

        clearscreen::clear().expect("Expected to clear screen!");
        match {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                if let Ok(user) = self
                    .rt
                    .block_on(async move { api.get_one_user(api.get_username().as_str()).await })
                {
                    Ok(user.is_smb)
                } else {
                    Err(NeptisError::Str("Failed to pull user info!".into()))
                }
            } else {
                Err(NeptisError::Str("API is not valid!".into()))
            }
        } {
            Ok(x) => {
                if x {
                    if Confirm::new(
                        "SMB is currently enabled for your account. Do you want to disable it?",
                    )
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    .unwrap_or(false)
                    {
                        match {
                            let m_api = &*self.api.read().unwrap();
                            if let Some(api) = m_api {
                                self.rt.block_on(async { api.disable_smb().await })
                            } else {
                                Err(NeptisError::Str("API is not valid!".into()))
                            }
                        } {
                            Ok(_) => {
                                println!("**** Successful. It may take several minutes to apply.")
                            }
                            Err(e) => {
                                println!(
                                    "**** An unexpected error has occurred. Refreshing in 5 secs: {}",
                                    e.to_string()
                                );
                            }
                        }
                    }
                } else {
                    if Confirm::new(
                        "SMB is currently disabled for your account. Do you want to enable it?",
                    )
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    .unwrap_or(false)
                    {
                        if let Some(smb_pass) = Password::new("Please enter any password to use")
                            .with_validator(required!())
                            .prompt_skippable()
                            .expect("Failed to show prompt!")
                        {
                            match {
                                let m_api = &*self.api.read().unwrap();
                                if let Some(api) = m_api {
                                    self.rt
                                        .block_on(async { api.enable_smb(smb_pass.as_str()).await })
                                        .map(|_| api.get_username().clone())
                                } else {
                                    Err(NeptisError::Str("API is not valid!".into()))
                                }
                            } {
                                Ok(user) => {
                                    // Attempt to set all SMB passwords here.
                                    println!(
                                        "**** Successful. It may take several minutes to apply.\nVisit '\\\\IP\\{}-<POINT NAME>-<DATA/REPO>' (all lowercase) on local SMB.\n",
                                        user
                                    );
                                    match handle_sync(&user, &smb_pass) {
                                        Ok(_) => println!("\n*** Password sync success!"),
                                        Err(_) => println!("\n*** Password sync failed!"),
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "**** An unexpected error has occurred. Refreshing in 5 secs: {}",
                                        e.to_string()
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. Refreshing in 5 secs: {}",
                    e.to_string()
                );
            }
        }
        thread::sleep(Duration::from_secs(2));
        self.show_dashboard();
    }

    #[cfg(not(unix))]
    #[allow(dead_code)]
    fn start_fuse(&self) {
        clearscreen::clear().expect("Expected to clear screen!");
        println!(
            "*** FUSE is not available on your platform. Please use SMB or the built-in browser instead."
        );
        thread::sleep(Duration::from_secs(3));
        self.show_dashboard();
    }

    #[cfg(unix)]
    fn start_fuse(&self, auto: bool) {
        use std::path::{Path, PathBuf};
        pub fn unmount_if_stale<P: AsRef<Path>>(path: P) {
            let path_ref = path.as_ref();

            if let Err(e) = fs::read_dir(path_ref) {
                if e.to_string()
                    .contains("Transport endpoint is not connected")
                {
                    // Try lazy unmount
                    let _ = std::process::Command::new("umount")
                        .arg("-l")
                        .arg(path_ref)
                        .status();
                }
            }
        }
        loop {
            clearscreen::clear().expect("Expected to clear screen!");
            let mnt = {
                let fuse_guard = self.fuse.lock().unwrap();
                fuse_guard.as_ref().map(|f| {
                    f.mountpoint
                        .to_str()
                        .expect("Expected mountpath to unwrap!")
                        .to_owned()
                })
            };

            if let Some(mnt) = mnt {
                if auto {
                    break;
                } else {
                    println!("\n*** FUSE is connected to {}", mnt);

                    if Confirm::new("Do you want to disable FUSE")
                        .with_default(false)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        .unwrap_or(false)
                    {
                        let mut fuse_guard = self.fuse.lock().unwrap();
                        *fuse_guard = None;
                    } else {
                        break;
                    }
                }
            } else {
                println!("*** FUSE is not enabled.");
                if auto
                    || Confirm::new("Do you want to enable FUSE")
                        .with_default(false)
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                        .unwrap_or(false)
                {
                    let d_path = self
                        .mnt
                        .clone()
                        .unwrap_or(get_working_dir().to_str().unwrap().to_string());
                    if auto && !d_path.is_empty() {
                        unmount_if_stale(&d_path);
                        match (|| {
                            let raw_path = PathBuf::from(&d_path);
                            if !raw_path.exists() {
                                fs::create_dir_all(raw_path)
                                    .map_err(|_| "Failed to create FUSE directory.".to_string())?;
                            } else if !raw_path.is_dir() {
                                return Err("FUSE directory is a file!".to_string());
                            }

                            let fs = NeptisFS::new(self.api.clone(), self.rt.clone());
                            Ok(
                                fuse_mt::spawn_mount(fuse_mt::FuseMT::new(fs, 1), &d_path, &[])
                                    .map_err(|e| e.to_string())?,
                            )
                        })() {
                            Ok(x) => {
                                let mut fuse_guard = self.fuse.lock().unwrap();
                                *fuse_guard = Some(x);
                            }
                            Err(e) => {
                                println!("> Failed to auto-mount: {e}");
                                thread::sleep(Duration::from_secs(2));
                            }
                        }
                        break;
                    } else if let Some(mnt_path) = Text::new("Please type an existing mount path")
                        .with_default(d_path.as_str())
                        .prompt_skippable()
                        .expect("Prompt failed to load!")
                    {
                        let _ = fs::create_dir_all(&mnt_path);
                        unmount_if_stale(&mnt_path);
                        let fs = NeptisFS::new(self.api.clone(), self.rt.clone());
                        match fuse_mt::spawn_mount(fuse_mt::FuseMT::new(fs, 1), mnt_path, &[]) {
                            Ok(x) => {
                                let mut fuse_guard = self.fuse.lock().unwrap();
                                *fuse_guard = Some(x);
                                println!("> Mount successful!");
                                thread::sleep(Duration::from_secs(2));
                                break;
                            }
                            Err(e) => {
                                println!("> Failed to mount: {e}");
                                thread::sleep(Duration::from_secs(2));
                                continue;
                            }
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.show_dashboard();
    }

    fn start_browser(&self) {
        FileBrowser::new(NeptisFS::new(self.api.clone(), self.rt.clone()))
            .show_browser(FileBrowserMode::Normal)
            .map(|_| ());
        self.show_dashboard();
    }

    fn send_message(&self, clear: bool) {
        if clear {
            clearscreen::clear().expect("Failed to clear screen!");
        }
        // Pull the list of all users for auto-complete.
        let u_ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                self.rt.block_on(async move { api.get_all_users().await })
            } else {
                Err(NeptisError::Str("API is invalid!".into()))
            }
        };
        match u_ret {
            Ok(users) => {
                let mut opts = users
                    .iter()
                    .map(|x| x.to_short_id_string())
                    .collect::<Vec<_>>();
                opts.insert(0, "Everyone".into());
                if let Some(sel_id) = Select::new("Send To", opts)
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                {
                    let sel_user = if sel_id == "Everyone" {
                        None
                    } else {
                        Some(
                            users
                                .iter()
                                .find(|x| x.to_short_id_string() == sel_id)
                                .map(|x| x.user_name.clone())
                                .expect("Expected ID to be available on user list!"),
                        )
                    };
                    if let Some(msg) = Text::new("Enter Message")
                        .with_validator(required!())
                        .prompt_skippable()
                        .expect("Failed to show prompt!")
                    {
                        // Send the message to the specified user.
                        println!("\n*** Sending message...");
                        if let Err(e) = {
                            let m_api = &*self.api.read().unwrap();
                            if let Some(api) = m_api {
                                self.rt.block_on(async move {
                                    api.send_one_message(PostForMessageApi {
                                        sent_to: sel_user,
                                        message: msg,
                                    })
                                    .await
                                })
                            } else {
                                Err(NeptisError::Str("API is invalid!".into()))
                            }
                        } {
                            println!(
                                "**** An unexpected error has occurred. ****\n{}",
                                e.to_string()
                            );
                            thread::sleep(Duration::from_secs(2));
                        }
                    }
                }
            }
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(2));
            }
        }
    }

    fn show_messages(&self) {
        use crossterm::{
            event::{self, Event, KeyCode},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                self.rt
                    .block_on(async move { api.get_all_messages(false).await })
            } else {
                Err(NeptisError::Str("API is invalid!".into()))
            }
        };
        match ret {
            Ok(messages) => {
                const PAGE_SIZE: usize = 10;
                let mut offset = messages.len().saturating_sub(PAGE_SIZE);

                loop {
                    clearscreen::clear().expect("Failed to clear screen!");
                    println!(
                        "*** Showing messages {} to {} ***\n",
                        offset + 1,
                        usize::min(offset + PAGE_SIZE, messages.len())
                    );

                    // Get a page of messages
                    let end = usize::min(offset + PAGE_SIZE, messages.len());
                    let page = &messages[offset..end];

                    // Display each message
                    for msg in page {
                        println!("{}\n", msg.to_short_id_string());
                    }

                    println!("\nUse left and right arrows to traverse | 's' to send | 'q' to exit");

                    enable_raw_mode().expect("Failed to enable raw mode");
                    let result = event::read();
                    disable_raw_mode().expect("Failed to disable raw mode");

                    match result {
                        Ok(Event::Key(key)) => {
                            if key.is_press() {
                                match key.code {
                                    KeyCode::Left => {
                                        offset = offset.saturating_sub(PAGE_SIZE);
                                    }
                                    KeyCode::Right => {
                                        if offset + PAGE_SIZE < messages.len() {
                                            offset += PAGE_SIZE;
                                        }
                                    }
                                    KeyCode::Char('q') | KeyCode::Enter => {
                                        break;
                                    }
                                    KeyCode::Char('s') => {
                                        // Ask about sending the message.
                                        self.send_message(true);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                println!(
                    "**** An unexpected error has occurred. ****\n{}",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(2));
            }
        }
        self.show_dashboard();
    }

    // inspected
    fn show_dashboard(&self) {
        use crossterm::{
            event::{self, Event},
            terminal::disable_raw_mode,
        };
        use std::{
            io::Write,
            process, thread,
            time::{Duration, Instant},
        };

        const STR_BROWSER: &str = "File Browser";

        #[cfg(unix)]
        const STR_FUSE: &str = "Start FUSE";

        const STR_SYNC: &str = "Manage Client-Side Sync";
        const STR_BREAKDOWN: &str = "Show Usage Breakdown";
        const STR_MESSAGE: &str = "View Messages";
        const STR_POINTS: &str = "Manage Points";
        const STR_USERS: &str = "Manage Users";
        const STR_NOTIFICATION: &str = "Manage Notifications";
        const STR_SYSTEM: &str = "Manage System";
        const STR_SMB: &str = "Manage SMB";
        const STR_PASSWORD: &str = "Change Password";
        const STR_LOGOUT: &str = "Logout";
        const STR_BACK: &str = "Go Back";
        let mut last_refresh = Instant::now();
        let mut first_time: bool = true;
        let mut is_admin: bool = false;
        loop {
            if first_time || last_refresh.elapsed().as_secs() >= 1000 {
                clearscreen::clear().expect("Failed to clear screen!");
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
                    // Check to see if the SMB is enabled or not.
                    let (s, admin) = self.get_luser_stats(api, false);
                    is_admin = admin;
                    println!("Connection: {}\n", api.to_string());
                    println!("{}", s);
                    if let Ok(ret) = self.rt.block_on(async { api.get_all_messages(true).await }) {
                        let r_len = ret.len();
                        if r_len > 0 {
                            println!(
                                "*** You have {} new message{}!",
                                r_len,
                                if r_len == 1 {
                                    String::new()
                                } else {
                                    "s".to_string()
                                }
                            );
                        }
                    }
                    println!(
                        "\n============== Top {} Jobs:\n{}\n\n",
                        MAX_JOBS,
                        match self
                            .rt
                            .block_on(async move { api.get_all_jobs(MAX_JOBS, Some(0)).await })
                            .map(|mut x| {
                                x.sort_by(|a, b| b.create_date.cmp(&a.create_date));
                                x.iter()
                                    .map(|x| x.to_short_id_string())
                                    .take(MAX_JOBS)
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }) {
                            Ok(ret) =>
                                if ret.trim().is_empty() {
                                    "None".into()
                                } else {
                                    ret.trim().to_string()
                                },
                            Err(_) => "Failed to load".into(),
                        }
                    );

                    // Attempt to show all running RCLONE jobs.
                    println!(
                        "============== Top {} Sync Jobs\n{}\n\n",
                        MAX_SYNC_JOBS,
                        match self
                            .rt
                            .block_on(async { WebApi::ipc_get_jobs().await })
                            .map(|mut x| {
                                x.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
                                x.iter()
                                    .map(|x| x.to_short_id_string())
                                    .take(MAX_SYNC_JOBS)
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }) {
                            Ok(ret) => {
                                if ret.trim().is_empty() {
                                    "None".into()
                                } else {
                                    ret.trim().to_string()
                                }
                            }
                            Err(_) => "*** Background service pull failed!".into(),
                        }
                    );
                } else {
                    disable_raw_mode().ok();
                    self.begin(); // not logged in
                    return;
                }
                println!("Press <ENTER> to show options...");
                last_refresh = Instant::now();
                first_time = false;
            }

            // Poll for a keypress non-blocking
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(k) = event::read().unwrap()
                    && k.is_press()
                {
                    break;
                }
            }
        }
        let mut menu_items = vec![STR_BACK, STR_BROWSER];

        #[cfg(unix)]
        {
            menu_items.push(STR_FUSE);
        }

        menu_items.push(STR_MESSAGE);
        menu_items.push(STR_BREAKDOWN);
        menu_items.push(STR_POINTS);

        if is_admin {
            menu_items.push(STR_USERS);
            menu_items.push(STR_SYSTEM);
        }

        menu_items.push(STR_SYNC);
        menu_items.push(STR_NOTIFICATION);
        menu_items.push(STR_PASSWORD);
        menu_items.push(STR_SMB);
        menu_items.push(STR_LOGOUT);

        // Show menu
        #[allow(unreachable_patterns)]
        match Select::new("Please select an action", menu_items)
            .prompt_skippable()
            .expect("Failed to show prompt!")
        {
            Some(STR_BROWSER) => self.start_browser(),

            #[cfg(unix)]
            Some(STR_FUSE) => self.start_fuse(false),

            Some(STR_BREAKDOWN) => self.show_point_breakdown(),
            Some(STR_POINTS) => self.show_points(),
            Some(STR_USERS) => self.show_users(),
            Some(STR_SYSTEM) => self.show_system(),
            Some(STR_PASSWORD) => self.show_change_password(),
            Some(STR_SMB) => self.show_smb(),
            Some(STR_SYNC) => self.show_rclone_schedules(),
            Some(STR_NOTIFICATION) => self.show_notifications(),
            Some(STR_MESSAGE) => self.show_messages(),
            Some(STR_BACK) => {
                clearscreen::clear().expect("Failed to clear screen!");
                self.show_dashboard();
            }
            Some(STR_LOGOUT) => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!("Logout\n");
                process::exit(0);
            }
            _ => {
                clearscreen::clear().expect("Failed to clear screen!");
                if Confirm::new("Do you want to logout?")
                    .with_default(false)
                    .prompt_skippable()
                    .expect("Failed to show prompt!")
                    .unwrap_or(true)
                {
                    clearscreen::clear().expect("Failed to clear screen!");
                    println!("Logout\n");
                    process::exit(0);
                }
            }
        }
    }

    fn show_connect(&self, server: ServerItem, auto: bool) {
        {
            let mut api = self.api.write().unwrap();
            let mut server = self.server.write().unwrap();
            *server = None;
            *api = None;
        }
        clearscreen::clear().unwrap();
        println!("Neptis Authentication");
        println!("Server IP: {}", server.server_endpoint.as_str());
        println!(
            "Server Encrypted: {}",
            if server.server_password.is_some() {
                "YES"
            } else {
                "NO"
            }
        );

        #[cfg(unix)]
        println!(
            "FUSE On Login: {}",
            if server.auto_fuse { "YES" } else { "NO" }
        );

        println!(
            "Arduino Enabled: {}\n",
            if server.arduino_endpoint.is_some() {
                "YES"
            } else {
                "NO"
            }
        );

        if !server.is_default {}

        fn prompt_password() -> String {
            Password::new("Password")
                .with_validator(required!())
                .without_confirmation()
                .prompt_skippable()
                .expect("Failed to show password prompt!")
                .unwrap_or("".into())
        }

        let (p_user, p_password) =
            if auto && server.user_name.is_some() && server.user_password.is_some() {
                let u = server.user_name.clone().unwrap();
                let p = server.user_password.clone().unwrap();
                println!("Using saved credentials for default server.");
                (u, p)
            } else {
                // Prompt for username
                let p_user = match Text::new("Username")
                    .with_validator(required!())
                    .with_initial_value(server.user_name.clone().unwrap_or_default().as_str())
                    .prompt_skippable()
                    .expect("Failed to show username prompt!")
                {
                    Some(x) => x,
                    None => {
                        self.begin();
                        return; // go back
                    }
                };

                // Prompt for password only if username matches saved one
                let p_password = match server.user_name.as_ref() {
                    Some(name) if name == &p_user => {
                        server.user_password.clone().unwrap_or_else(prompt_password)
                    }
                    _ => prompt_password(),
                };

                (p_user, p_password)
            };

        if p_password.is_empty() {
            self.begin();
            return;
        }

        // Attempt to connect to the server.
        let pass = server.server_password.as_ref();
        let mut secret = None;
        let mut raw_connect_func = || {
            if let Some(s_text) = pass {
                secret = Some(
                    RollingSecret::from_string(s_text.as_str())
                        .ok_or("Failed to parse secret!".to_string())?,
                );
            }
            let t_api = WebApi::new(
                server.server_endpoint.as_str(),
                p_user.clone(),
                p_password.clone(),
                secret.clone(),
            );
            self.rt
                .block_on(async { t_api.get_info().await })
                .map_err(|e| format!("Failed to load server: {e}"))?;
            Ok(t_api)
        };

        let mut ret: Result<WebApi, String> = raw_connect_func();
        // At this point - attempt to see if the Arduino can wake up the PC.
        use crossterm::{
            event::{self, Event, KeyCode},
            execute,
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::io::stdout;
        use std::thread;
        use std::time::{Duration, Instant};
        if ret.is_err() {
            let mut good = true;
            if let Err(ref e) = ret {
                if e.to_lowercase().contains("auth") {
                    good = false;
                }
            }
            if good {
                if let Some(a_endpoint) = server.arduino_endpoint.clone() {
                    if let Some(a_pass) = server.arduino_password.clone() {
                        println!("Initial connection failed. Attempting to wake up PC...");

                        let ep = a_endpoint.as_str();
                        let a_func = || {
                            let key = ArduinoSecret::from_string(a_pass.as_str())
                                .ok_or("Failed to parse Arduino Key".to_string())?
                                .rolling_key()
                                .ok_or("Failed to calculate next Arduino key".to_string())?;
                            ClientBuilder::new()
                                .build()
                                .map(|x| {
                                    self.rt.block_on(async move {
                                        x.post(format!("{}/{}", ep, "start"))
                                            .bearer_auth(key.to_string())
                                            .send()
                                            .await
                                            .ok()
                                    })
                                })
                                .ok()
                                .flatten()
                                .ok_or("Failed to send packet to Arduino!".to_string())?
                                .error_for_status()
                                .map_err(|e| {
                                    format!("Arduino returned error upon submission: {}", e)
                                })
                                .map(|_| ())
                        };

                        let mut sig_good = false;
                        for _ in 0..3 {
                            match a_func() {
                                Ok(_) => {
                                    println!(
                                        "Successfully sent signal. Waiting for server to respond..."
                                    );
                                    sig_good = true;
                                    break;
                                }
                                Err(e) => println!("Failed to send signal: {e}. Trying again..."),
                            }
                            thread::sleep(Duration::from_secs(2));
                        }

                        if sig_good {
                            println!(
                                "Retrying connection for up to 2 minutes... Press any key to cancel."
                            );
                            enable_raw_mode().ok();
                            let start_time = Instant::now();
                            let mut user_cancelled = false;

                            loop {
                                if let Ok(true) = event::poll(Duration::from_secs(1)) {
                                    if let Ok(Event::Key(k)) = event::read()
                                        && k.is_press()
                                    {
                                        user_cancelled = true;
                                        break;
                                    }
                                }

                                disable_raw_mode().ok();
                                println!("Retrying...");
                                enable_raw_mode().ok();
                                ret = raw_connect_func();
                                if ret.is_ok() {
                                    break;
                                }

                                if start_time.elapsed() > Duration::from_secs(120) {
                                    break;
                                }

                                // Wait 10 seconds with cancellation check
                                for _ in 0..10 {
                                    if let Ok(true) = event::poll(Duration::from_secs(1)) {
                                        if let Ok(Event::Key(k)) = event::read()
                                            && k.is_press()
                                        {
                                            user_cancelled = true;
                                            break;
                                        }
                                    }
                                }

                                if user_cancelled {
                                    break;
                                }
                            }

                            disable_raw_mode().ok();

                            if user_cancelled {
                                println!("Cancelled by user.");
                            } else if ret.is_ok() {
                                println!("Reconnected successfully.");
                            } else {
                                println!("Timed out after 2 minutes of retries.");
                            }
                        } else {
                            ret = raw_connect_func(); // One last attempt
                        }
                    }
                }
            }
        }
        match ret {
            Ok(x) => {
                {
                    let mut api = self.api.write().unwrap();
                    *api = Some(x);
                    let mut serv = self.server.write().unwrap();
                    *serv = Some(server.clone());
                }
                println!("Connection successful!");
                #[cfg(unix)]
                {
                    if server.auto_fuse {
                        self.start_fuse(true);
                    }
                }

                self.show_dashboard();
            }
            Err(e) => {
                println!("Failed to connect to server. Error: {}", e.to_string());
                thread::sleep(Duration::from_secs(3));
                if auto {
                    // Prevent an infinite loop by terminating.
                    process::exit(1);
                } else {
                    self.begin();
                }
            }
        }
    }

    pub fn begin(&self) {
        use crossterm::{
            event::{self, Event},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::time::Duration;

        clearscreen::clear().unwrap();
        fn format_slash(s: &str) -> String {
            s.strip_suffix("/").unwrap_or(s).to_string()
        }

        // 7-1-25: Check to see if the IPC service is running or not.
        if cfg!(not(debug_assertions))
            && let Ok(schedules) = self.db.get_all_transfer_auto_schedules_sync()
            && schedules.len() > 0
            && self
                .rt
                .block_on(async { WebApi::ipc_ping().await.is_err() })
            && Confirm::new("The background service is not running. Do you want to exit?")
                .with_default(false)
                .prompt_skippable()
                .expect("Failed to show prompt!")
                .map(|x| if x { None } else { Some(x) })
                .flatten()
                .is_none()
        {
            return;
        }

        // Check if a default server is set.
        if cfg!(not(debug_assertions)) {
            if let Some(d_item) = self
                .db
                .get_all_servers_sync()
                .map(|x| x.into_iter().find(|x| x.is_default))
                .expect("Failed to pull from database")
            {
                let mut do_auto = true;
                println!(
                    "Connecting to {} ({}) in 2 seconds...",
                    d_item.server_name.as_str(),
                    d_item.server_endpoint.as_str()
                );
                enable_raw_mode().expect("Failed to enable raw mode");
                let start_date = Instant::now();
                loop {
                    if event::poll(Duration::from_millis(100)).expect("Polling failed") {
                        if let Event::Key(k) = event::read().expect("Failed to read event")
                            && k.is_press()
                        {
                            do_auto = false;
                            break;
                        }
                    }
                    if start_date.elapsed().as_secs() >= 2 {
                        break;
                    }
                    thread::yield_now();
                }
                disable_raw_mode().expect("Failed to disable raw mode");
                if do_auto {
                    self.show_connect(d_item, true);
                }
            }
        }

        self.show_connect(
            ModelManager::new(
                Some(&self.db),
                vec![
                    ModelProperty::new(
                        "Server Name",
                        true,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Please enter Server Name")
                                .with_initial_value(serv.server_name.as_str())
                                .with_validator(required!())
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.server_name = x;
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.server_name.clone(),
                    ),
                    ModelProperty::new(
                        "Server Endpoint",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Server URL")
                                .with_initial_value(serv.server_endpoint.as_str())
                                .with_validator(|input: &str| {
                                    if input.trim().is_empty() {
                                        return Ok(Validation::Invalid(
                                            "This field is required.".into(),
                                        ));
                                    }
                                    match Url::parse(input) {
                                        Ok(_) => Ok(Validation::Valid),
                                        Err(_) => Ok(Validation::Invalid(
                                            "Please enter a valid URL.".into(),
                                        )),
                                    }
                                })
                                .with_formatter(&format_slash)
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.server_endpoint = x;
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.server_endpoint.clone(),
                    ),
                    ModelProperty::new(
                        "Server Password",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Server Password")
                                .with_initial_value(&serv.server_password.clone().unwrap_or("".into()))
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.server_password = if x.is_empty() {
                                        None
                                    } else {
                                        Some(x.trim().to_string())
                                    };
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| {
                            x.server_password
                                .clone()
                                .map(|_| "[FILLED]".to_string())
                                .unwrap_or("[EMPTY]".to_string())
                        },
                    ),
                    ModelProperty::new(
                        "Default User",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Default User")
                                .with_initial_value(&serv.user_name.clone().unwrap_or("".into()))
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.user_name = if x.is_empty() {
                                        None
                                    } else {
                                        Some(x.trim().to_string())
                                    };
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.user_name.clone().unwrap_or("[EMPTY]".to_string()),
                    ),
                    ModelProperty::new(
                        "Default User Password",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Default User Password")
                                .with_initial_value(&serv.user_password.clone().unwrap_or("".into()))
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.user_password = if x.is_empty() {
                                        None
                                    } else {
                                        Some(x.trim().to_string())
                                    };
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| {
                            x.user_password
                                .clone()
                                .map(|_| "[FILLED]".to_string())
                                .unwrap_or("[EMPTY]".to_string())
                        },
                    ),
                    ModelProperty::new(
                        "Arduino Endpoint",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Arduino Endpoint")
                                .with_initial_value(&serv.arduino_endpoint.clone().unwrap_or("".into()))
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.arduino_endpoint = if x.is_empty() {
                                        None
                                    } else {
                                        Some(x.trim().to_string())
                                    };
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.arduino_endpoint.clone().unwrap_or("[EMPTY]".to_string()),
                    ),
                    ModelProperty::new(
                        "Arduino Password",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Text::new("Enter Arduino Password")
                                .with_initial_value(&serv.arduino_password.clone().unwrap_or("".into()))
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.arduino_password = if x.is_empty() {
                                        None
                                    } else {
                                        Some(x.trim().to_string())
                                    };
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| {
                            x.arduino_password
                                .clone()
                                .map(|_| "[FILLED]".to_string())
                                .unwrap_or("[EMPTY]".to_string())
                        },
                    ),
                    ModelProperty::new_for_linux_only(
                        "Auto Fuse",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Confirm::new("Do you want FUSE to auto-mount")
                                .with_default(serv.auto_fuse)
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.auto_fuse = x;
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.auto_fuse.to_string()),
                    ModelProperty::new(
                        "Set As Default",
                        false,
                        |_, serv: &mut ServerItem| {
                            match Confirm::new("Do you want the server to be default (will replace others)")
                                .with_default(serv.is_default)
                                .prompt_skippable()
                                .expect("Failed to show prompt!") {
                                Some(x) => {
                                    serv.is_default = x;
                                    PromptResult::Ok
                                },
                                None => PromptResult::Cancel
                            }
                        },
                        |x| x.is_default.to_string())
                ],
                Box::new(|ctx| Ok(ctx.api.expect("Expected DB to be valid!").get_all_servers_sync()?)),
            )
                .with_select_title(format!(
                    "Neptis Front End v{}\nCopyright (c) 2025 Eric E. Gold\nThis software is licensed under GPLv3\n",
                    env!("CARGO_PKG_VERSION")
                ))
                .with_modify(Box::new(|ctx, servers, serv| {
                    let mut m_servers = servers.clone();
                    if serv.is_default {
                        for server in m_servers.iter_mut() {
                            // Overwrite default status for other servers.
                            server.is_default = false;
                        }
                    }

                    if let Some(u_serv) = m_servers
                        .iter_mut()
                        .find(|x| x.server_name == serv.server_name.as_str())
                    {
                        *u_serv = serv.clone();
                    } else {
                        m_servers.push(serv.clone());
                    }
                    Ok(ctx.api.expect("Expected DB to be valid!").overwrite_servers_sync(m_servers.as_slice())?)
                }))
                .with_delete(Box::new(|ctx, x| {
                    Ok(ctx.api.expect("Expected DB to be valid!").delete_server_sync(x.server_name.as_str())?)
                }))
                .do_display()
                .expect("Failed to show information!")
                .expect("Expected server to be selected!"),
            false,
        );
    }

    #[cfg(unix)]
    pub fn new(mnt: Option<String>) -> UiApp {
        let rt = Arc::new(Runtime::new().expect("Expected Runtime to start!"));
        let db = DbController::new(rt.clone());
        UiApp {
            rt: rt.clone(),
            api: Arc::new(RwLock::new(None)),
            fuse: Mutex::new(None),
            db,
            mnt,
            server: RwLock::new(None),
        }
    }

    #[cfg(not(unix))]
    pub fn new() -> UiApp {
        let rt = Arc::new(Runtime::new().expect("Expected Runtime to start!"));
        let db = DbController::new(rt.clone());
        UiApp {
            rt: rt.clone(),
            api: Arc::new(RwLock::new(None)),
            server: RwLock::new(None),
            db,
        }
    }
}

use clap::{ArgGroup, Parser};
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use inquire::formatter::StringFormatter;
use itertools::Itertools;
use neptis_rs::db::sync_models::TransferJobDto;
use neptis_rs::db::sync_models::TransferJobStatus;
use neptis_rs::get_working_dir;
use neptis_rs::prelude::{
    AlertMode, AlertTrigger, ArduinoSecret, AutoJobDto, AutoJobType, DbController, FileSize,
    JobStatus, JobType, NeptisError, NeptisFS, PostForAutoScheduleStartDto, PostForMessageApi,
    PostForSubscriptionApi, PutForAutoJobWebApi, PutForMountApi, PutForSubscriptionApi, RepoJobDto,
    ServerItem, SnapshotFileDto, SubscriptionDto, TransferAutoJob, TransferAutoSchedule, UserDto,
    UserForCreateApi, UserForUpdateApi, WebApi,
};
use neptis_rs::rolling_secret::RollingSecret;
use neptis_rs::traits::ToShortIdString;
use neptis_rs::ui::browser::{FileBrowser, FileBrowserMode};
use neptis_rs::ui::manager::{ModelManager, ModelProperty, PromptResult};
use rocket::futures::SinkExt;

#[derive(Parser, Debug)]
#[command(name = "Neptis")]
#[command(about = "Neptis Front End", long_about = None)]
pub struct CliArgs {
    /// Prevent updates from running
    #[arg(long = "no-update")]
    pub no_update: Option<bool>,

    /// Set default FUSE mount path
    #[cfg(unix)]
    #[arg(long = "default-fuse", value_name = "PATH", env = "NEPTIS_MNT")]
    pub default_fuse: Option<String>,

    /// Use beta/pre-release updates instead of stable
    #[arg(long = "beta", conflicts_with = "no_update")]
    pub beta: Option<bool>,
}

pub fn main() {
    let args = CliArgs::parse();
    clearscreen::clear().expect("Expected to clear screen!");

    if !args.no_update.unwrap_or(false) {
        println!("*** Checking for updates...");
        match (|| {
            AxoUpdater::new_for("neptis-rs")
                .load_receipt()?
                .configure_version_specifier(if args.beta.unwrap_or(false) {
                    UpdateRequest::LatestMaybePrerelease
                } else {
                    UpdateRequest::Latest
                })
                .run_sync()
        })() {
            Ok(x) => {
                if let Some(ret) = x {
                    println!("Successfully updated to {}", ret.new_version.to_string())
                } else {
                    println!("No updates found")
                }
            }
            Err(e) => {
                println!("FAILED to pull updates. Error: {e}");
            }
        }

        thread::sleep(Duration::from_millis(500));
    }

    #[cfg(unix)]
    let app = UiApp::new(args.default_fuse);

    #[cfg(not(unix))]
    let app = UiApp::new();

    app.begin();
}
