#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate core;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

pub mod apis;
pub mod error;
pub mod models;
pub mod rolling_secret;
pub mod arduino_secret;
pub mod ui;
pub mod filesystem;
pub mod macros;

use crate::ui::file_size::FileSize;

use apis::dtos::{AutoJobType, JobStatus, JobType, PutForAutoJobWebApi, RepoJobDto};
use arduino_secret::ArduinoSecret;
use chrono::{Local, Utc};
use cron::Schedule;
use filesystem::NeptisFS;
use fuser::BackgroundSession;
use inquire::Editor;
use reqwest::ClientBuilder;
use ui::file_browser::TreeBrowser;
use uuid::Uuid;
use std::ffi::OsStr;
use std::process;
use std::str::FromStr;

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

use apis::{
    NeptisError,
    dtos::{AutoJobDto, PutForMountApi, SnapshotFileDto},
};
use inquire::{Confirm, CustomType, Password, Select, Text, required, validator::Validation};
use models::SnapshotDto;
use rolling_secret::RollingSecret;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use ui::{
    manager::{
        ApiContext, ModelExtraOption, ModelManager, ModelProperty, PropGetType, ToShortIdString,
    },
    server::ServerItem,
};
use url::Url;

use crate::apis::api::*;

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

struct UiApp {
    rt: Arc<Runtime>,
    api: Arc<RwLock<Option<WebApi>>>,
    fuse: Mutex<Option<BackgroundSession>>,
}

static DEFAULT_PASS: &'static str = "default123";
static MAX_JOBS: usize = 15;

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
            println!("Locked:           {}", if snapshot.locked { "YES" } else { "NO" });
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
                        if index > 0 {
                            index -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if index + 1 < snapshots.len() {
                            index += 1;
                        }
                    },
                    KeyCode::Char('l') => {
                        // Attempt to lock or unlock the given snapshot.
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
                        }.is_ok() {
                            snapshot.locked = !is_locked;
                            continue;
                        }
                    },
                    KeyCode::Char('q') | KeyCode::Enter => {
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        self.on_manage_snapshot(mount);
    }

    // inspected
    fn on_select_job(&self, mount: &str, j_id: Uuid) {
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
            // Refresh job details every 10 seconds
            if first_time || last_refresh.elapsed().as_secs() >= 10 {
                first_time = true;
                clearscreen::clear().expect("Failed to clear screen!");
                let result = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        self.rt.block_on(async move {
                            api.get_all_jobs_for_mount(mount).await?
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
                        println!("Snapshot ID:       {}", dto.snapshot_id.as_deref().unwrap_or("-"));
                        println!("Point Owned By:    {}", dto.point_owned_by);
                        println!("Point Name:        {}", dto.point_name);
                        println!("Job Type:          {}", dto.job_type.to_string());
                        println!("Job Status:        {}", dto.job_status.to_string());
                        println!("Used Bytes:        {}", FileSize::prettify(dto.used_bytes as u64));
                        println!(
                            "Total Bytes:       {}",
                            dto.total_bytes
                                .map(|x| FileSize::prettify(x as u64))
                                .unwrap_or("-".into())
                        );
                        if !dto.errors.is_empty() {
                            println!("Errors ({}):        {}", dto.errors.len(), dto.errors.join(", "));
                        } else {
                            println!("Errors:            -");
                        }
                        println!("Create Date:       {}", dto.create_date.format("%Y-%m-%d %H:%M:%S"));
                        println!(
                            "End Date:          {}",
                            dto.end_date
                                .map(|x| x.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or("-".to_string())
                        );
                        println!("=================================================\n");
                        println!("(Waiting... Press any key to select options or exit)\n");
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
                        if key_event.code == KeyCode::Enter {
                            break;
                        }
    
                        // Go to interactive menu
                        let dto = {
                            let m_api = &*self.api.read().unwrap();
                            if let Some(api) = m_api {
                                self.rt.block_on(async move {
                                    api.get_all_jobs_for_mount(mount).await?
                                        .into_iter()
                                        .find(|x| x.id == j_id)
                                        .ok_or(NeptisError::Str("Failed to find the job!".into()))
                                })
                            } else {
                                Err(NeptisError::Str("API is not valid!".into()))
                            }
                        };
    
                        match dto {
                            Ok(dto) => {
                                if dto.snapshot_id.is_some() {
                                    if Select::new("Please select an action", vec!["Go Back", "View Snapshot"])
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                        == "View Snapshot"
                                    {
                                        match {
                                            let m_api = &*self.api.read().unwrap();
                                            if let Some(api) = m_api {
                                                self.rt.block_on(async move {
                                                    api.get_one_snapshot(
                                                        dto.point_name.as_str(),
                                                        dto.snapshot_id
                                                            .clone()
                                                            .expect("Expected snapshot to be valid!")
                                                            .as_str(),
                                                    )
                                                    .await
                                                })
                                            } else {
                                                Err(NeptisError::Str("API is not valid!".into()))
                                            }
                                        } {
                                            Ok(s_dto) => self.on_select_snapshot(mount, &[s_dto]),
                                            Err(_) => break,
                                        }
                                    } else {
                                        break;
                                    }
                                } else {
                                    if Confirm::new("Do you want to go back")
                                        .with_default(true)
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                    {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    _ => {}
                }
            }
        }
    
        self.on_view_jobs(mount, None);
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
                        |_: &mut RepoJobDto| panic!("Not allowed to modify job"),
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
                Some(dto) => self.on_select_job(mount, dto.id),
                None => {
                        self.on_select_mount(mount)

                }
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
                    .prompt()
                    .expect("Failed to show prompt!")
                {
                    break;
                }
            }
        }
        self.on_manage_autojobs(mount);
    }

    // inspected
    fn on_manage_autojobs(&self, mount: &str) {
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
                            |dto: &mut AutoJobDto| {
                                dto.task_name = Text::new("Please enter Task Name")
                                    .with_validator(required!())
                                    .with_initial_value(dto.task_name.as_str())
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.task_name.clone(),
                        ),
                        ModelProperty::new(
                            "Cron Schedule",
                            false,
                            |dto: &mut AutoJobDto| {
                                dto.cron_schedule = Text::new("Please enter Cron Schedule")
                                    .with_validator(required!())
                                    .with_validator(|s: &str| match Schedule::from_str(s) {
                                        Ok(_) => Ok(Validation::Valid),
                                        Err(_) => Ok(Validation::Invalid(
                                            "Cron schedule is not valid!".into(),
                                        )),
                                    })
                                    .with_initial_value(dto.cron_schedule.as_str())
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.cron_schedule.clone(),
                        ),
                        ModelProperty::new(
                            "Enabled",
                            false,
                            |dto: &mut AutoJobDto| {
                                dto.enabled = Confirm::new("Do you want it Enabled")
                                    .with_default(dto.enabled)
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.enabled.to_string(),
                        ),
                        ModelProperty::new(
                            "Job Type",
                            false,
                            |dto: &mut AutoJobDto| {
                                dto.job_type = CustomType::<AutoJobType>::new(
                                    "Please enter Job Type (Backup/Check)",
                                )
                                .with_starting_input(
                                    dto.job_type.to_string().replace("Unknown", "").as_str(),
                                )
                                .prompt()
                                .expect("Failed to show prompt!")
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

    fn get_snapshot_mm<'a>(api: &'a WebApi, mount: &str) -> ModelManager<'a, SnapshotFileDto> {
        let mount_owned = mount.to_string(); // make it owned
        ModelManager::new(
            Some(api),
            vec![ModelProperty::new(
                "ID",
                true,
                |_: &mut SnapshotFileDto| panic!("Not allowed to modify snapshot"),
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
                    ctx.rt
                        .block_on(async move { 
                            let mut ret = api.get_all_snapshots(&mount_inner).await?;
                            ret.sort_by_key(|x|std::cmp::Reverse(x.time));
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
                JobType::Backup => Confirm::new("Do you want to lock the snapshot").prompt().expect("Failed to show prompt!"),
                JobType::Restore => Confirm::new("Do you want to overwrite data").prompt().expect("Failed to show prompt!"),
                _ => false
            };

            if Confirm::new("Do you want to proceed")
                .prompt()
                .expect("Failed to show prompt!")
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
                                    api.post_one_restore(mount, s_ret.unwrap().id.as_str(), opt)
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
                        self.on_select_job(mount, x.id.clone());
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
                    api.get_one_user(&api.get_username()).await.ok().map(|x|x.is_smb)
                })
            }
            (stats, smb.unwrap_or(false))
        } {
            (Some(stats), smb) => {
                let d_total = stats.data_max_bytes;
                let d_used = stats.data_used_bytes;
                let d_free = d_used.map(|x| d_total - x);

                let r_total = stats.repo_max_bytes;
                let r_used = stats.repo_used_bytes;
                let r_free = r_used.map(|x| r_total - x);

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
                    println!("**** SMB Enabled at: \\\\IP\\{}-{}-<data or repo>\n", stats.owned_by.as_str(), stats.name.as_str());
                }
                println!("Created At: {}", stats.date_created.and_utc().with_timezone(&Local).format("%Y-%m-%d %I:%M:%S %p").to_string());
                println!("Data Accessed At: {}", stats.data_accessed.and_utc().with_timezone(&Local).format("%Y-%m-%d %I:%M:%S %p").to_string());
                println!("Repo Accessed At: {}", stats.repo_accessed.and_utc().with_timezone(&Local).format("%Y-%m-%d %I:%M:%S %p").to_string());
                println!();
                println!("Data Usage: {}", prettify_bytes(d_total, d_used, d_free));
                println!("Repo Usage: {}", prettify_bytes(r_total, r_used, r_free));
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
        .prompt()
        .expect("Failed to show prompt!")
        {
            STR_MANAGE_SNAPSHOT => self.on_manage_snapshot(mount),
            STR_MANAGE_JOB => self.on_view_jobs(mount, None),
            STR_MANAGE_AUTO_JOB => self.on_manage_autojobs(mount),
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
            .prompt()
            .expect("Failed to show prompt!")
        {
            let a_str = format!("Please enter password for {}", user.user_name.as_str());
            let p = Password::new(a_str.as_str())
                .with_validator(required!())
                .prompt()
                .expect("Failed to show prompt!");
            match (|| {
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
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
                                    password: Some(p),
                                },
                            )
                            .await
                        })
                        .map(|_| ())
                } else {
                    Err(NeptisError::Str("API is invalid!".into()))
                }
            })() {
                Ok(_) => {
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
        self.show_users();
    }

    fn get_luser_stats(&self, api: &WebApi, is_breakdown: bool) -> String {
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
                                    y.data_used_bytes.unwrap_or(0) as u64,
                                    y.data_max_bytes as u64
                                ))
                                .collect();
                            let mut repo_points: Vec<(String, u64, u64)> = mounts.iter()
                                .map(|y| (
                                    y.name.clone(),
                                    y.repo_used_bytes.unwrap_or(0) as u64,
                                    y.repo_max_bytes as u64
                                ))
                                .collect();
    
                            // Sort by total (used bytes) descending
                            data_points.sort_by(|a, b| b.1.cmp(&a.1));
                            repo_points.sort_by(|a, b| b.1.cmp(&a.1));
    
                            let data_total = mounts.iter().map(|y| y.data_max_bytes).sum::<i64>() as u64;
                            let repo_total = mounts.iter().map(|y| y.repo_max_bytes).sum::<i64>() as u64;
                            let data_used = mounts.iter().map(|y| y.data_used_bytes.unwrap_or(0)).sum::<i64>() as u64;
                            let repo_used = mounts.iter().map(|y| y.repo_used_bytes.unwrap_or(0)).sum::<i64>() as u64;
    
                            // Format breakdown
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
                                user.max_data_bytes
                                    .map(|x| FileSize::prettify(x as u64))
                                    .unwrap_or("N/A".into()),
                                data_breakdown,
                                FileSize::prettify(repo_total),
                                user.max_snapshot_bytes
                                    .map(|x| FileSize::prettify(x as u64))
                                    .unwrap_or("N/A".into()),
                                repo_breakdown
                            )
                        } else {
                            // Original behavior
                            let (d_max, r_max, d_used, r_used) = (
                                FileSize::prettify(mounts.iter().map(|y| y.data_max_bytes).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.repo_max_bytes).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.data_used_bytes.unwrap_or(0)).sum::<i64>() as u64),
                                FileSize::prettify(mounts.iter().map(|y| y.repo_used_bytes.unwrap_or(0)).sum::<i64>() as u64),
                            );
                            format!(
                                "Data Point Allocation: {d_max} / {}\nData Point File Usage: {d_used} / {d_max}\n\nRepo Point Allocation: {r_max} / {}\nRepo Point File Usage: {r_used} / {r_max}",
                                user.max_data_bytes
                                    .map(|x| FileSize::prettify(x as u64))
                                    .unwrap_or("N/A".into()),
                                user.max_snapshot_bytes
                                    .map(|x| FileSize::prettify(x as u64))
                                    .unwrap_or("N/A".into())
                            )
                        }
                    })
                })
                .unwrap_or("Failed to calculate Data Total File Usage".into());
            format!(
                "Logged in as {}\nPrivledged: {}\n{}",
                user.user_name.as_str(),
                if user.is_admin { "Yes" } else { "No" },
                usage_str
            )
        } else {
            "Failed to calculate User Information".into()
        }
    }

    // inspected
    fn show_points(&self) {
        let ret = {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                // Attempt to pull the maximum user statistics
                let stats = self.get_luser_stats(api, false);
                println!("{}\n", stats.as_str());
                ModelManager::new(
                    Some(api),
                    vec![
                        ModelProperty::new(
                            "Name",
                            true,
                            |dto: &mut InternalMountDto| {
                                dto.name = Text::new("Please enter Mount Name")
                                    .with_initial_value(dto.name.as_str())
                                    .with_validator(required!())
                                    .with_validator(|m_name: &str|{
                                        if !regex::Regex::new(r"^[a-z_][a-z0-9_-]*$")
                                            .expect("Expected regex to work")
                                            .is_match(m_name) 
                                        {
                                            Ok(Validation::Invalid("Bad name!".into()))
                                        } else {
                                            Ok(Validation::Valid)
                                        }
                                    })
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.name.clone(),
                        ),
                        ModelProperty::new(
                            "Data Bytes",
                            false,
                            |dto: &mut InternalMountDto| {
                                let si = FileSize::from_bytes(dto.data_bytes as u64).to_string();
                                dto.data_bytes =
                                    CustomType::<FileSize>::new("Please enter maximum data size")
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
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                        .get_bytes() as i64
                            },
                            |x| FileSize::from_bytes(x.data_bytes as u64).to_string(),
                        ),
                        ModelProperty::new(
                            "Repo Bytes",
                            false,
                            |dto: &mut InternalMountDto| {
                                let si = FileSize::from_bytes(dto.repo_bytes as u64).to_string();
                                dto.repo_bytes =
                                    CustomType::<FileSize>::new("Please enter maximum repo size")
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
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                        .get_bytes() as i64
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
                                        data_bytes: x.data_max_bytes,
                                        repo_bytes: x.repo_max_bytes,
                                    })
                                    .collect::<Vec<_>>()
                            })
                        })
                    }),
                )
                .with_create_title(stats.clone())
                .with_modify_title(stats.clone())
                .with_back()
                .with_delete(Box::new(|ctx, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt
                        .block_on(async move { api.delete_one_mount(dto.name.as_str()).await })
                }))
                .with_modify(Box::new(|ctx, _, dto| {
                    let api = ctx
                        .api
                        .as_deref()
                        .ok_or(NeptisError::Str("API is not valid!".into()))?;
                    ctx.rt
                        .block_on(async move {
                            api.put_one_mount(
                                dto.name.as_str(),
                                PutForMountApi {
                                    data_bytes: dto.data_bytes,
                                    repo_bytes: dto.repo_bytes,
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
                            |user: &mut UserDto| {
                                user.user_name = Text::new("Please enter Username")
                                    .with_initial_value(user.user_name.as_str())
                                    .with_validator(required!())
                                    .with_validator(|m_name: &str|{
                                        if !regex::Regex::new(r"^[a-z_][a-z0-9_-]*$")
                                            .expect("Expected regex to work")
                                            .is_match(m_name) 
                                        {
                                            Ok(Validation::Invalid("Bad name!".into()))
                                        } else {
                                            Ok(Validation::Valid)
                                        }
                                    })
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.user_name.clone(),
                        ),
                        ModelProperty::new(
                            "First Name",
                            false,
                            |user: &mut UserDto| {
                                user.first_name = Text::new("Please enter First Name")
                                    .with_initial_value(user.first_name.as_str())
                                    .with_validator(required!())
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.first_name.clone(),
                        ),
                        ModelProperty::new(
                            "Last Name",
                            false,
                            |user: &mut UserDto| {
                                user.last_name = Text::new("Please enter Last Name")
                                    .with_initial_value(user.last_name.as_str())
                                    .with_validator(required!())
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.last_name.clone(),
                        ),
                        ModelProperty::new(
                            "Is Admin",
                            false,
                            |user: &mut UserDto| {
                                user.is_admin = Confirm::new("Should the user be admin")
                                    .with_default(user.is_admin)
                                    .prompt()
                                    .expect("Failed to show prompt!")
                            },
                            |x| x.is_admin.to_string(),
                        ),
                        ModelProperty::new(
                            "Max Data",
                            false,
                            |user: &mut UserDto| {
                                let si =
                                    FileSize::from(user.max_data_bytes.unwrap_or(0)).to_string();
                                user.max_data_bytes = Some(
                                    CustomType::<FileSize>::new("Please enter maximum data size")
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
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                        .get_bytes() as i64,
                                )
                            },
                            |x| {
                                x.max_data_bytes
                                    .map(|x| FileSize::from_bytes(x as u64).to_string())
                                    .unwrap_or("N/A".into())
                            },
                        ),
                        ModelProperty::new(
                            "Max Repo",
                            false,
                            |user: &mut UserDto| {
                                let si = FileSize::from(user.max_snapshot_bytes.unwrap_or(0))
                                    .to_string();
                                user.max_snapshot_bytes = Some(
                                    CustomType::<FileSize>::new("Please enter maximum repo size")
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
                                        .prompt()
                                        .expect("Failed to show prompt!")
                                        .get_bytes() as i64,
                                )
                            },
                            |x| {
                                x.max_snapshot_bytes
                                    .map(|x| FileSize::from_bytes(x as u64).to_string())
                                    .unwrap_or("N/A".into())
                            },
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
                                max_data_bytes: dto.max_data_bytes.clone(),
                                max_snapshot_bytes: dto.max_snapshot_bytes.clone(),
                            })
                            .await
                        } else {
                            api.put_one_user(
                                dto.user_name.as_str(),
                                UserForUpdateApi {
                                    first_name: Some(dto.first_name.clone()),
                                    last_name: Some(dto.last_name.clone()),
                                    is_admin: Some(dto.is_admin),
                                    max_data_bytes: dto.max_data_bytes.clone(),
                                    max_snapshot_bytes: dto.max_snapshot_bytes.clone(),
                                    password: None, // password will be set seperately
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
        let mut running_jobs = vec![];
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
                running_jobs = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        self.rt
                            .block_on(async { api.get_all_jobs(MAX_JOBS, Some(0)).await.unwrap_or(vec![]) })
                    } else {
                        self.show_dashboard();
                        return;
                    }
                }
                .into_iter()
                .filter(|x| x.job_status == JobStatus::Running)
                .collect::<Vec<_>>();

                if running_jobs.len() > 0 {
                    println!(
                        "***** WARNING: Shutdown / Restart is unsafe due to {} job(s) running!",
                        running_jobs.len()
                    );
                }
                println!("\n\nPress <ENTER> to show options...");
                last_refresh = Instant::now();
            }  
    
            // Poll for a keypress non-blocking
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(_) = event::read().unwrap() {
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
        .prompt()
        .expect("Failed to show prompt!");

        fn handle_unsafe(jobs: &[RepoJobDto]) -> bool {
            let j_count = jobs.iter().count();
            if j_count <= 0 {
                return true;
            }
            clearscreen::clear().expect("Failed to clear screen!");
            println!(
                "**** DANGER: Powering down the system will corrupt {} job(s):",
                j_count
            );
            for job in jobs {
                println!("{}", job.to_short_id_string());
            }
            println!("\n");
            Confirm::new("Please confirm your decision")
                .prompt()
                .expect("Failed to show prompt!")
        }

        match choice {
            STR_REFRESH => self.show_system(),
            STR_SHUTDOWN => {
                if !handle_unsafe(running_jobs.as_slice()) {
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
                if !handle_unsafe(running_jobs.as_slice()) {
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
       

    // inspected
    fn show_change_password(&self) {
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                let p = Password::new("Please enter your new password")
                    .with_validator(required!())
                    .prompt()
                    .expect("Failed to show prompt!");
                match self
                    .rt
                    .block_on(async move { api.put_password(p.as_str()).await })
                {
                    Ok(_) => println!("**** Successfully changed password!"),
                    Err(_) => println!("**** Failed to change password!"),
                }
                thread::sleep(Duration::from_secs(2));
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
                    println!("{}", self.get_luser_stats(api, true));
                } else {
                    println!("Failed to load additional statistics.");
                }
            }
            println!("\n\n\n");
            if Confirm::new("Do you want to go back").with_default(true).prompt().expect("Failed to show prompt!") {
                break;
            }
        }
        self.show_dashboard();
    }

    fn show_browser(&self) {
        // We need to lock the API, then return the file browsing instance.
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                let _ = TreeBrowser::new(api, &self.rt).run();
            }
        }
        self.show_dashboard();
    }

    fn show_smb(&self) {
        clearscreen::clear().expect("Expected to clear screen!");
        match {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                if let Ok(user) = self.rt.block_on(async move {
                    api.get_one_user(api.get_username().as_str()).await
                }) {
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
                    if Confirm::new("SMB is currently enabled for your account. Do you want to disable it?").prompt().expect("Failed to show prompt!") {
                        match {
                            let m_api = &*self.api.read().unwrap();
                            if let Some(api) = m_api {
                                self.rt.block_on(async {
                                    api.disable_smb().await
                                })
                            } else {
                                Err(NeptisError::Str("API is not valid!".into()))
                            }
                        } {
                            Ok(_) => println!("**** Successful. It may take several minutes to apply."),
                            Err(e) => {
                                println!("**** An unexpected error has occurred. Refreshing in 5 secs: {}", e.to_string());
                            }
                        }
                    }
                } else {
                    if Confirm::new("SMB is currently disabled for your account. Do you want to enable it?").prompt().expect("Failed to show prompt!") {
                        let smb_pass = Password::new("Please enter any password to use").with_validator(required!()).prompt().expect("Failed to show prompt!");
                        match {
                            let m_api = &*self.api.read().unwrap();
                            if let Some(api) = m_api {
                                self.rt.block_on(async {
                                    api.enable_smb(smb_pass.as_str()).await
                                }).map(|_|api.get_username().clone())
                            } else {
                                Err(NeptisError::Str("API is not valid!".into()))
                            }
                        } {
                            Ok(user) => println!("**** Successful. It may take several minutes to apply.\nVisit '\\\\IP\\{}-<POINT NAME>-<DATA/REPO>' (all lowercase) on local SMB.", user),
                            Err(e) => {
                                println!("**** An unexpected error has occurred. Refreshing in 5 secs: {}", e.to_string());
                            }
                        }
                    }
                }
            },
            Err(e) => {
                println!("**** An unexpected error has occurred. Refreshing in 5 secs: {}", e.to_string());
            }
        }
        thread::sleep(Duration::from_secs(2));
        self.show_dashboard();
    }

    fn start_fuse(&self) {
        loop {
            clearscreen::clear().expect("Expected to clear screen!");
            let mnt = {
                let fuse_guard = self.fuse.lock().unwrap();
                fuse_guard.as_ref().map(|f| f.mountpoint.to_str().expect("Expected mountpath to unwrap!").to_owned())
            };
        
            if let Some(mnt) = mnt {
                println!("\n*** FUSE is connected to {}", mnt);
        
                if Confirm::new("Do you want to disable FUSE")
                    .with_default(false)
                    .prompt()
                    .expect("Failed to show prompt!")
                {
                    let mut fuse_guard = self.fuse.lock().unwrap();
                    *fuse_guard = None;
                } else {
                    break;
                }
            } else {
                println!("*** FUSE is not enabled.");
                if Confirm::new("Do you want to enable FUSE")
                    .with_default(false)
                    .prompt()
                    .expect("Failed to show prompt!")
                {
                    let d_path = format!("{}/neptis-mnt", dirs_next::home_dir()
                        .expect("Expected home directory!")
                        .to_str()
                        .expect("Expected directory to parse!"));
                    let mnt_path = Text::new("Please type a mount name")
                        .with_default(d_path.as_str())
                        .prompt()
                        .expect("Prompt failed to load!");
    
                    let fs = NeptisFS::new(self.api.clone(), self.rt.clone());
                    match fuse_mt::spawn_mount(fuse_mt::FuseMT::new(fs, 1), mnt_path, &[]) {
                        Ok(x) => {
                            let mut fuse_guard = self.fuse.lock().unwrap();
                            *fuse_guard = Some(x);
                            println!("> Mount successful!");
                            thread::sleep(Duration::from_secs(2));
                            break;
                        },
                        Err(e) => {
                            println!("> Failed to mount: {e}");
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    }       
                } else {
                    break;
                }
            }
        }
        self.show_dashboard();
    }
    

    // inspected
    fn show_dashboard(&self) {
        use crossterm::{
            event::{self, Event},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::{
            io::Write,
            process, thread,
            time::{Duration, Instant},
        };

    
        const STR_FUSE: &str = "Start FUSE";
        const STR_BROWSE: &str = "File Browser";
        const STR_BREAKDOWN: &str = "Show Usage Breakdown";
        const STR_POINTS: &str = "Manage Points";
        const STR_USERS: &str = "Manage Users";
        const STR_SYSTEM: &str = "Manage System";
        const STR_SMB: &str = "Manage SMB";
        const STR_PASSWORD: &str = "Change Password";
        const STR_LOGOUT: &str = "Logout";
        const STR_BACK: &str = "Go Back";
        let mut last_refresh = Instant::now();
        let mut first_time: bool = true;
        loop {
            if first_time || last_refresh.elapsed().as_secs() >= 1000 {
                first_time = false;
                clearscreen::clear().expect("Failed to clear screen!");
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
                    // Check to see if the SMB is enabled or not.
                    
                    println!("Connection: {}\n", api.to_string());
                    println!("{}", self.get_luser_stats(api, false));
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
                } else {
                    disable_raw_mode().ok();
                    self.begin(); // not logged in
                    return;
                }
                println!("Press <ENTER> to show options...");
                last_refresh = Instant::now();
            }

            // Poll for a keypress non-blocking
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(_) = event::read().unwrap() {
                    break;
                }
            }
        }
        // Show menu with "Go Back"
        match inquire::Select::new(
            "Please select an action",
            vec![
                STR_BACK,
                STR_FUSE,
                STR_BROWSE,
                STR_BREAKDOWN,
                STR_POINTS,
                STR_USERS,
                STR_SYSTEM,
                STR_PASSWORD,
                STR_SMB,
                STR_LOGOUT,
            ],
        )
        .prompt()
        .expect("Failed to show prompt!")
        {
            STR_FUSE => self.start_fuse(),
            STR_BREAKDOWN => self.show_point_breakdown(),
            STR_BROWSE => self.show_browser(),
            STR_POINTS => self.show_points(),
            STR_USERS => self.show_users(),
            STR_SYSTEM => self.show_system(),
            STR_PASSWORD => self.show_change_password(),
            STR_SMB => self.show_smb(),
            STR_BACK => {
                // Call show_dashboard again to resume auto-refresh
                clearscreen::clear().expect("Failed to clear screen!");
                self.show_dashboard();
            },
            STR_LOGOUT | _ => {
                clearscreen::clear().expect("Failed to clear screen!");
                println!("Logout\n");
                process::exit(0);
            }
        }
    }

    fn show_connect(&self, server: ServerItem) {
        {
            let mut api = self.api.write().unwrap();
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
        println!("Arduino Enabled: {}\n", 
            if server.arduino_endpoint.is_some() {
                "YES"
            } else { 
                "NO" 
            }
        );
        let p_user = Text::new("Username")
            .with_validator(required!())
            .with_initial_value(server.user_name.clone().unwrap_or(String::new()).as_str())
            .prompt()
            .expect("Failed to show username prompt!");
        let p_password = Password::new("Password")
            .with_validator(required!())
            .without_confirmation()
            .prompt()
            .expect("Failed to show password prompt!");

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
            let mut t_api =
                WebApi::new(server.server_endpoint.as_str(), p_user.clone(), p_password.clone(), secret.clone());
            self.rt
                .block_on(async { t_api.get_info().await })
                .ok()
                .ok_or("Failed to load server")?;
            Ok(t_api)
        };

        let mut ret: Result<WebApi, String> = raw_connect_func();
        // At this point - attempt to see if the Arduino can wake up the PC.
        if ret.is_err() {
            if let Some(a_endpoint) = server.arduino_endpoint {
                if let Some(a_pass) = server.arduino_password {
                    println!("Initial connection failed. Attempting to wake up PC...");
                    if let Err(e) = (||{
                        let key = ArduinoSecret::from_string(a_pass.as_str())
                            .ok_or(
                                "Failed to parse Arduino Key".to_string(),
                            )?
                            .rolling_key()
                            .ok_or(
                                "Failed to calculate next Arduino key".to_string(),
                            )?;
                        ClientBuilder::new()
                            .build()
                            .map(|x| {
                                self.rt.block_on(async move {
                                    x.post(format!("{}/{}", a_endpoint, "start"))
                                        .bearer_auth(key.to_string())
                                        .send()
                                        .await
                                        .ok()
                                })
                            })
                            .ok()
                            .flatten()
                            .ok_or(
                                "Failed to send packet to Arduino!".to_string(),
                            )?
                            .error_for_status()
                            .map_err(|e|
                                format!("Arduino returned error upon submission: {}", e),
                            ).map(|_|())
                    })() {
                        println!("*** Failed to send Arduino signal: {}", e);
                    }
                    // No matter what - try one more time in-case something works now.
                    ret = raw_connect_func();
                }
            }
        }
        match ret {
            Ok(x) => {
                {
                    let mut api = self.api.write().unwrap();
                    *api = Some(x)
                }
                println!("Connection successful!");
                self.show_dashboard();
            }
            Err(e) => {
                println!("Failed to connect to server. Error: {}", e.to_string());
                thread::sleep(Duration::from_secs(3));
                self.begin();
            }
        }
    }

    pub fn begin(&self) {
        clearscreen::clear().unwrap();
        println!("Neptis Login");
        fn format_slash(s: &str) -> String {
            s.strip_suffix("/").unwrap_or(s).to_string()
        }

        self.show_connect(
            ModelManager::new(
                None,
                vec![
                    ModelProperty::new(
                        "Server Name",
                        true,
                        |serv: &mut ServerItem| {
                            serv.server_name = Text::new("Please enter Server Name")
                                .with_initial_value(serv.server_name.as_str())
                                .with_validator(required!())
                                .prompt()
                                .expect("Failed to show prompt!")
                        },
                        |x| x.server_name.clone(),
                    ),
                    ModelProperty::new(
                        "Server Endpoint",
                        false,
                        |serv: &mut ServerItem| {
                            serv.server_endpoint = Text::new("Enter Server URL")
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
                                .prompt()
                                .expect("Failed to show prompt!");
                        },
                        |x| x.server_endpoint.clone(),
                    ),
                    ModelProperty::new(
                        "Server Password",
                        false,
                        |serv: &mut ServerItem| {
                            serv.server_password = match Editor::new("Re-type Server Password")
                                .prompt()
                                .expect("Failed to show prompt!")
                                .as_str()
                                .trim()
                            {
                                "" => None,
                                x => Some(x.to_string()),
                            };
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
                        |serv: &mut ServerItem| {
                            serv.user_name = match Text::new("Enter Default User")
                                .with_initial_value(
                                    serv.user_name.clone().unwrap_or("".into()).as_str(),
                                )
                                .prompt()
                                .expect("Failed to show prompt!")
                                .as_str()
                                .trim()
                            {
                                "" => None,
                                x => Some(x.to_string()),
                            };
                        },
                        |x| x.user_name.clone().unwrap_or("[EMPTY]".to_string()),
                    ),
                    ModelProperty::new(
                        "Arduino Endpoint",
                        false,
                        |serv: &mut ServerItem| {
                            serv.arduino_endpoint = match Text::new("Enter Arduino Endpoint")
                                .with_initial_value(
                                    serv.arduino_endpoint.clone().unwrap_or("".into()).as_str(),
                                )
                                .prompt()
                                .expect("Failed to show prompt!")
                                .as_str()
                                .trim()
                            {
                                "" => None,
                                x => Some(x.to_string()),
                            };
                        },
                        |x| x.arduino_endpoint.clone().unwrap_or("[EMPTY]".to_string()),
                    ),
                    ModelProperty::new(
                        "Arduino Password",
                        false,
                        |serv: &mut ServerItem| {
                            serv.arduino_password = match Editor::new("Re-type Arduino Password")
                                .prompt()
                                .expect("Failed to show prompt!")
                                .as_str()
                                .trim()
                            {
                                "" => None,
                                x => Some(x.to_string()),
                            };
                        },
                        |x| {
                            x.arduino_password
                                .clone()
                                .map(|_| "[FILLED]".to_string())
                                .unwrap_or("[EMPTY]".to_string())
                        },
                    ),
                ],
                Box::new(|_| ServerItem::load_servers()),
            )
            .with_modify(Box::new(|_, servers, serv| {
                let mut m_servers = servers.clone();
                if let Some(u_serv) = m_servers
                    .iter_mut()
                    .find(|x| x.server_name == serv.server_name.as_str())
                {
                    *u_serv = serv.clone();
                } else {
                    m_servers.push(serv.clone());
                }
                ServerItem::save_servers(m_servers.as_slice())
            }))
            .with_delete(Box::new(|_, x| {
                ServerItem::delete_server(x.server_name.as_str())
            }))
            .do_display()
            .expect("Failed to show information!")
            .expect("Expected server to be selected!"),
        );
    }


    pub fn new() -> UiApp {
        UiApp {
            rt: Arc::new(Runtime::new().expect("Failed to start runtime!")),
            api: Arc::new(RwLock::new(None)),
            fuse: Mutex::new(None),
        }
    }
}

pub fn main() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let app = UiApp::new();
    app.begin();
}