use core::fmt;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::traits::ToShortIdString;

use super::{NeptisError, api::PointUsage};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutoJobType {
    #[default]
    Unknown,
    Backup,
    Check,
}

impl FromStr for AutoJobType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backup" => Ok(AutoJobType::Backup),
            "check" => Ok(AutoJobType::Check),
            _ => Err(()),
        }
    }
}

impl ToString for AutoJobType {
    fn to_string(&self) -> String {
        match self {
            AutoJobType::Backup => "Backup",
            AutoJobType::Check => "Check",
            AutoJobType::Unknown => "Unknown",
        }
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JobType {
    #[default]
    Unknown,
    Backup,
    Restore,
    Check,
    Prune,
    PointAdjust,
    PointDelete,
}

impl ToString for JobType {
    fn to_string(&self) -> String {
        match self {
            JobType::Unknown => "Unknown",
            JobType::Backup => "Backup",
            JobType::Check => "Check",
            JobType::Restore => "Restore",
            JobType::Prune => "Prune",
            JobType::PointAdjust => "PointAdjust",
            JobType::PointDelete => "PointDelete",
        }
        .into()
    }
}

impl FromStr for JobType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unknown" => Ok(JobType::Unknown),
            "backup" => Ok(JobType::Backup),
            "restore" => Ok(JobType::Restore),
            "check" => Ok(JobType::Check),
            "prune" => Ok(JobType::Prune),
            "pointadjust" => Ok(JobType::PointAdjust),
            "pointdelete" => Ok(JobType::PointDelete),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JobStatus {
    #[default]
    Unknown,
    NotStarted,
    Running,
    Successful,
    Failed,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::Unknown => "Unknown",
            JobStatus::Failed => "Failed",
            JobStatus::Running => "Running",
            JobStatus::Successful => "Successful",
            JobStatus::NotStarted => "Not Started",
        }
        .into()
    }
}

impl FromStr for JobStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unknown" => Ok(JobStatus::Unknown),
            "failed" => Ok(JobStatus::Failed),
            "running" => Ok(JobStatus::Running),
            "successful" => Ok(JobStatus::Successful),
            "notstarted" => Ok(JobStatus::NotStarted),
            "not started" => Ok(JobStatus::NotStarted),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize)]

pub struct MountDto {
    pub name: String,
    pub owned_by: String,
    pub usage: PointUsage,
    pub date_created: NaiveDateTime,
    pub data_accessed: NaiveDateTime,
    pub repo_accessed: NaiveDateTime,
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RepoJobDto {
    pub id: Uuid,
    pub title: Option<String>,
    pub snapshot_id: Option<String>,
    pub point_owned_by: String,
    pub point_name: String,
    pub job_type: JobType,
    pub job_status: JobStatus,
    pub used_bytes: i64,
    pub total_bytes: Option<i64>,
    pub errors: Vec<String>,
    pub create_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
    pub auto_job: Option<String>,
}

impl ToShortIdString for RepoJobDto {
    fn to_short_id_string(&self) -> String {
        let status_display = match self.job_status {
            JobStatus::Running => match self.total_bytes {
                Some(total) if total > 0 => {
                    let percent = (self.used_bytes as f64 / total as f64 * 100.0).round() as u32;
                    format!("{:>16}", format!("{}% COMPLETE", percent))
                }
                _ => format!("{:>16}", "PROGRESS UNAVAILABLE"),
            },
            _ => format!("{:<16}", self.job_status.to_string().to_uppercase()),
        };

        format!(
            "{}: {} for ({} {} on {}) ({}) | {} errors",
            format!(
                "{:<8}",
                self.id.to_string().chars().take(8).collect::<String>()
            ),
            format!(
                "{:<16}",
                self.point_name
                    .to_string()
                    .chars()
                    .take(16)
                    .collect::<String>()
            ),
            status_display,
            format!("{:<8}", self.job_type.to_string()),
            self.create_date
                .and_utc()
                .with_timezone(&Local)
                .format("%Y-%m-%d %I:%M:%S %p")
                .to_string(),
            self.auto_job
                .clone()
                .map(|x| format!("AUTO: {}", x))
                .unwrap_or("USER".into()),
            self.errors.len()
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PostForAutoScheduleStartDto {
    pub server_name: String,
    pub schedule_name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AutoJobDto {
    pub task_name: String,
    pub cron_schedule: String,
    pub enabled: bool,
    pub date_created: NaiveDateTime,
    pub date_modified: NaiveDateTime,
    pub date_last_ran: Option<NaiveDateTime>,
    pub job_type: AutoJobType,
}

impl ToShortIdString for AutoJobDto {
    fn to_short_id_string(&self) -> String {
        use cron_descriptor::cronparser::cron_expression_descriptor;
        format!(
            "{} ({})",
            self.task_name.clone(),
            cron_expression_descriptor::get_description_cron(self.cron_schedule.as_str())
                .unwrap_or("N/A".into())
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDto {
    pub path: String,
    pub atime: NaiveDateTime,
    pub ctime: NaiveDateTime,
    pub mtime: NaiveDateTime,
    pub is_dir: bool,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PutForAutoJobWebApi {
    pub task_name: String,
    pub cron_schedule: String,
    pub job_type: AutoJobType,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PutForFileApi {
    pub path: String,
    pub base64: Option<String>,
    pub new_path: Option<String>,
    pub atime: Option<NaiveDateTime>,
    pub mtime: Option<NaiveDateTime>,
    pub offset: Option<u64>,
    pub t_len: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PutForXattrApi {
    pub path: String,
    pub key: String,
    pub base64: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeleteForXattrApi {
    pub path: String,
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct PostForFileApi {
    pub path: String,
    pub is_dir: bool,
    pub base64: Option<String>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeekPos {
    Start(u64),
    End(i64),
    Current(i64),
}

impl From<SeekPos> for std::io::SeekFrom {
    fn from(pos: SeekPos) -> Self {
        match pos {
            SeekPos::Start(n) => std::io::SeekFrom::Start(n),
            SeekPos::End(n) => std::io::SeekFrom::End(n),
            SeekPos::Current(n) => std::io::SeekFrom::Current(n),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PutForMountApi {
    pub data_bytes: i64,
    pub repo_bytes: i64,
}

#[derive(Serialize, Deserialize)]
pub struct PostForBackupApi {
    pub point_user: String,
    pub point_name: String,
    pub tags: Option<Vec<String>>,
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PostForRestoreApi {
    pub point_user: String,
    pub point_name: String,
    pub snapshot: String,
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GenericAutoJobManageApi {
    pub point_user: String,
    pub point_name: String,
    pub task_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GenericSnapshotManageApi {
    pub point_user: String,
    pub point_name: String,
    pub snapshot: String,
}

#[derive(Serialize, Deserialize)]
pub struct GenericRepoManageApi {
    pub point_user: String,
    pub point_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SnapshotFileDto {
    pub time: NaiveDateTime,
    pub program_version: String,
    pub parent: Option<String>,
    pub tree: String,
    pub label: String,
    pub paths: Vec<String>,
    pub tags: Vec<String>,
    pub original: Option<String>,
    pub summary: Option<SnapshotSummary>,
    pub description: Option<String>,
    pub id: String,
    pub locked: bool,
}

impl ToShortIdString for SnapshotFileDto {
    fn to_short_id_string(&self) -> String {
        format!(
            "{}{} on {}",
            self.id.to_string(),
            if self.locked { " <LOCKED> " } else { "" },
            self.summary
                .clone()
                .map(|x| x
                    .backup_end
                    .to_utc()
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %I:%M:%S %p")
                    .to_string())
                .unwrap_or("N/A".into())
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SnapshotSummary {
    /// New files compared to the last (i.e. parent) snapshot
    pub files_new: u64,

    /// Changed files compared to the last (i.e. parent) snapshot
    pub files_changed: u64,

    /// Unchanged files compared to the last (i.e. parent) snapshot
    pub files_unmodified: u64,

    /// Total processed files
    pub total_files_processed: u64,

    /// Total size of all processed files
    pub total_bytes_processed: u64,

    /// New directories compared to the last (i.e. parent) snapshot
    pub dirs_new: u64,

    /// Changed directories compared to the last (i.e. parent) snapshot
    pub dirs_changed: u64,

    /// Unchanged directories compared to the last (i.e. parent) snapshot
    pub dirs_unmodified: u64,

    /// Total processed directories
    pub total_dirs_processed: u64,

    /// Total number of data blobs added by this snapshot
    pub total_dirsize_processed: u64,

    /// Total size of all processed dirs
    pub data_blobs: u64,

    /// Total number of tree blobs added by this snapshot
    pub tree_blobs: u64,

    /// Total uncompressed bytes added by this snapshot
    pub data_added: u64,

    /// Total bytes added to the repository by this snapshot
    pub data_added_packed: u64,

    /// Total uncompressed bytes (new/changed files) added by this snapshot
    pub data_added_files: u64,

    /// Total bytes for new/changed files added to the repository by this snapshot
    pub data_added_files_packed: u64,

    /// Total uncompressed bytes (new/changed directories) added by this snapshot
    pub data_added_trees: u64,

    /// Total bytes (new/changed directories) added to the repository by this snapshot
    pub data_added_trees_packed: u64,

    /// The command used to make this backup
    pub command: String,

    /// Start time of the backup.
    ///
    /// # Note
    ///
    /// This may differ from the snapshot `time`.
    pub backup_start: DateTime<Local>,

    /// The time that the backup has been finished.
    pub backup_end: DateTime<Local>,

    /// Total duration of the backup in seconds, i.e. the time between `backup_start` and `backup_end`
    pub backup_duration: f64,

    /// Total duration that the rustic command ran in seconds
    pub total_duration: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PostForSubscriptionApi {
    pub mode: AlertMode,
    pub endpoint: String,
    pub triggers: Vec<AlertTrigger>,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PutForSubscriptionApi {
    pub mode: Option<AlertMode>,
    pub endpoint: Option<String>,
    pub triggers: Option<Vec<AlertTrigger>>,
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum AlertMode {
    #[default]
    Discord,
    Email,
    Post,
}

impl ToString for AlertMode {
    fn to_string(&self) -> String {
        match self {
            AlertMode::Discord => "Discord",
            AlertMode::Email => "Email",
            AlertMode::Post => "Post",
        }
        .into()
    }
}

impl FromStr for AlertMode {
    type Err = NeptisError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "discord" => Ok(AlertMode::Discord),
            "email" => Ok(AlertMode::Email),
            "post" => Ok(AlertMode::Post),
            _ => Err(NeptisError::Str("Failed to convert!".into())),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum AlertTrigger {
    #[default]
    UserMessage,
    PointCreated,
    PointModified,
    PointDeleted,
    JobStarted,
    JobFinished,
    JobError,
    ServerShutdown,
    ServerRestart,
    AutoJobCreated,
    AutoJobModified,
    AutoJobDeleted,
    SnapshotLocked,
    SnapshotUnlocked,
    SnapshotDeleted,
}

// Display: enables to_string()
impl Display for AlertTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            AlertTrigger::UserMessage => "UserMessage",
            AlertTrigger::PointCreated => "PointCreated",
            AlertTrigger::PointModified => "PointModified",
            AlertTrigger::PointDeleted => "PointDeleted",
            AlertTrigger::JobStarted => "JobStarted",
            AlertTrigger::JobFinished => "JobFinished",
            AlertTrigger::JobError => "JobError",
            AlertTrigger::ServerShutdown => "ServerShutdown",
            AlertTrigger::ServerRestart => "ServerRestart",
            AlertTrigger::AutoJobCreated => "AutoJobCreated",
            AlertTrigger::AutoJobModified => "AutoJobModified",
            AlertTrigger::AutoJobDeleted => "AutoJobDeleted",
            AlertTrigger::SnapshotLocked => "SnapshotLocked",
            AlertTrigger::SnapshotUnlocked => "SnapshotUnlocked",
            AlertTrigger::SnapshotDeleted => "SnapshotDeleted",
        };
        write!(f, "{}", s)
    }
}

// FromStr: enables "UserMessage".parse::<AlertTrigger>()
impl FromStr for AlertTrigger {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UserMessage" => Ok(AlertTrigger::UserMessage),
            "PointCreated" => Ok(AlertTrigger::PointCreated),
            "PointModified" => Ok(AlertTrigger::PointModified),
            "PointDeleted" => Ok(AlertTrigger::PointDeleted),
            "JobStarted" => Ok(AlertTrigger::JobStarted),
            "JobFinished" => Ok(AlertTrigger::JobFinished),
            "JobError" => Ok(AlertTrigger::JobError),
            "ServerShutdown" => Ok(AlertTrigger::ServerShutdown),
            "ServerRestart" => Ok(AlertTrigger::ServerRestart),
            "AutoJobCreated" => Ok(AlertTrigger::AutoJobCreated),
            "AutoJobModified" => Ok(AlertTrigger::AutoJobModified),
            "AutoJobDeleted" => Ok(AlertTrigger::AutoJobDeleted),
            "SnapshotLocked" => Ok(AlertTrigger::SnapshotLocked),
            "SnapshotUnlocked" => Ok(AlertTrigger::SnapshotUnlocked),
            "SnapshotDeleted" => Ok(AlertTrigger::SnapshotDeleted),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SubscriptionDto {
    pub id: Uuid,
    pub mode: AlertMode,
    pub endpoint: String,
    pub triggers: Vec<AlertTrigger>,
    pub enabled: bool,
}

impl ToShortIdString for SubscriptionDto {
    fn to_short_id_string(&self) -> String {
        format!(
            "{}{} ({}) - {} trigger(s)",
            if self.enabled { "" } else { "(DISABLED) " },
            self.mode.to_string(),
            self.endpoint,
            self.triggers.len()
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PostForMessageApi {
    pub sent_to: Option<String>,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub sent_from: String,
    pub sent_to: Option<String>,
    pub subject: Option<String>,
    pub message: String,
    pub sent_date: NaiveDateTime,
    pub read_by: Vec<String>,
    pub important: bool,
}

impl ToShortIdString for Message {
    fn to_short_id_string(&self) -> String {
        format!(
            "{}'{}'\n(by {} at {})",
            if let Some(ref sub) = self.subject {
                format!("{} ->\n", sub)
            } else {
                "".into()
            },
            &self.message,
            &self.sent_from,
            &self
                .sent_date
                .and_utc()
                .with_timezone(&Local)
                .format("%Y-%m-%d %I:%M:%S %p")
                .to_string()
        )
    }
}
