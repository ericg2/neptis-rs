use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use crate::prelude::TransferJobInternalDto;
use crate::traits::ToShortIdString;

impl From<TransferJob> for TransferJobInternalDto {
    fn from(value: TransferJob) -> Self {
        value.dto
    }
}

// const FAIL_MESSAGE: &'static str =
//     "Job cannot be recovered due to server data loss. Did it restart?";

impl From<TransferJobInternalDto> for TransferJob {
    fn from(value: TransferJobInternalDto) -> Self {
        TransferJob {
            dto: value,
            _thread: None,
            _cancel_rx: None,
            _cancel_tx: None,
        } // todo: add something for fail message here?
    }
}

impl AsRef<TransferJob> for TransferJob {
    fn as_ref(&self) -> &TransferJob {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RCloneLogLevel {
    #[serde(alias = "notice")]
    #[serde(alias = "info")]
    Notice,

    #[serde(alias = "warn")]
    #[serde(alias = "warning")]
    Warning,

    #[serde(alias = "err")]
    #[serde(alias = "error")]
    #[serde(alias = "fatal")]
    Error,

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneMessage {
    pub level: RCloneLogLevel,
    pub msg: String,
    pub stats: Option<RCloneStat>,
    pub time: DateTime<FixedOffset>,
}

fn float_or_int_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let val: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(u)
            } else if let Some(f) = n.as_f64() {
                Ok(f as u64) // truncate float to integer
            } else {
                Err(serde::de::Error::custom("Invalid number"))
            }
        }
        _ => Err(serde::de::Error::custom("Expected a number")),
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RCloneStat {
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub bytes: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub speed: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub checks: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub deletes: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub listed: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub renames: u64,
    pub retry_error: bool,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub deleted_dirs: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub server_side_copies: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub server_side_copy_bytes: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub server_side_move_bytes: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub server_side_moves: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub total_bytes: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub total_checks: u64,
    #[serde(deserialize_with = "float_or_int_to_u64")]
    pub total_transfers: u64,
}

pub struct TransferJob {
    pub dto: TransferJobInternalDto,
    pub _thread: Option<JoinHandle<()>>,
    pub _cancel_tx: Option<Sender<()>>,
    pub _cancel_rx: Option<Receiver<bool>>,
}

impl TransferJob {
    pub fn status(&self) -> TransferJobStatus {
        if self._thread.is_some() {
            TransferJobStatus::Running
        } else if self.dto.fatal_errors.len() > 0 {
            TransferJobStatus::Failed
        } else if self.dto.last_stats.is_none() {
            TransferJobStatus::NotStarted
        } else {
            TransferJobStatus::Successful
        }
    }
}

impl ToShortIdString for TransferJobDto {
    fn to_short_id_string(&self) -> String {
        let status_display = match self.stat {
            TransferJobStatus::Running => {
                if let Some(stats) = &self.last_stats {
                    if stats.total_bytes > 0 {
                        let percent = (stats.bytes as f64 / stats.total_bytes as f64 * 100.0).round() as u32;
                        format!("{:>16}", format!("{}% COMPLETE", percent))
                    } else {
                        format!("{:>16}", "PROGRESS UNAVAILABLE")
                    }
                } else {
                    format!("{:>16}", "NO STATS")
                }
            }
            _ => format!("{:<16}", format!("{:?}", self.stat).to_uppercase()),
        };

        let start_time = self
            .start_date
            .unwrap_or(self.last_updated)
            .and_utc()
            .with_timezone(&Local)
            .format("%Y-%m-%d %I:%M:%S %p")
            .to_string();

        format!(
            "{}: {} => {} | {} on {} | {} errors",
            format!(
                "{:<8}",
                self.job_id.to_string().chars().take(8).collect::<String>()
            ),
            format!(
                "{:<12}",
                self.server_name.chars().take(12).collect::<String>()
            ),
            format!(
                "{:<24}",
                self.smb_folder.chars().take(24).collect::<String>()
            ),
            format!("{:<24}", status_display),
            start_time,
            self.errors.len()
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum TransferJobStatus {
    NotStarted,
    Running,
    Successful,
    Failed,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TransferJobDto {
    pub job_id: Uuid,
    pub server_name: String,
    pub smb_folder: String,
    pub local_folder: String,
    pub stat: TransferJobStatus,
    pub errors: Vec<String>,
    pub last_stats: Option<RCloneStat>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub last_updated: NaiveDateTime,
    pub auto_job_schedule_name: Option<String>,
    pub auto_job_action_name: Option<String>,
}

impl <T: AsRef<TransferJob>> From<T> for TransferJobDto {
    fn from(job: T) -> Self {
        let job_ref = job.as_ref();
        let job_dto = &job_ref.dto;
        let stat = job_ref.status();
        TransferJobDto {
            job_id: job_dto.job_id.clone(),
            server_name: job_dto.server_name.clone(),
            smb_folder: job_dto.smb_folder.clone(),
            local_folder: job_dto.local_folder.clone(),
            errors: job_dto.fatal_errors.0.clone(),
            last_stats: job_dto.last_stats.clone().map(|x|x.0),
            start_date: job_dto.start_date.clone(),
            end_date: job_dto.end_date.clone(),
            last_updated: job_dto.last_updated.clone(),
            auto_job_action_name: job_dto.auto_job_action_name.clone(),
            auto_job_schedule_name: job_dto.auto_job_schedule_name.clone(),
            stat
        }
    }
}