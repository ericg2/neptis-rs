/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 * 
 * Generated by: https://openapi-generator.tech
 */

use crate::models;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoJobConfigDto {
    #[serde(rename = "dataPoint", skip_serializing_if = "Option::is_none")]
    pub data_point: Option<Box<models::DataPointDto>>,
    #[serde(rename = "repoPoint", skip_serializing_if = "Option::is_none")]
    pub repo_point: Option<Box<models::RepoPointDto>>,
    #[serde(rename = "name", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub name: Option<Option<String>>,
    #[serde(rename = "createdBy", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Option<String>>,
    #[serde(rename = "lastRan", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub last_ran: Option<Option<String>>,
    #[serde(rename = "cronSchedule", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub cron_schedule: Option<Option<String>>,
    #[serde(rename = "backupPath", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<Option<String>>,
    #[serde(rename = "nextRun", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub next_run: Option<Option<String>>,
    #[serde(rename = "dateCreated", skip_serializing_if = "Option::is_none")]
    pub date_created: Option<String>,
    #[serde(rename = "pastJobs", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub past_jobs: Option<Option<Vec<models::RepoDataJobDto>>>,
    #[serde(rename = "currentJob", skip_serializing_if = "Option::is_none")]
    pub current_job: Option<Box<models::RepoDataJobDto>>,
    #[serde(rename = "isRunning", skip_serializing_if = "Option::is_none")]
    pub is_running: Option<bool>,
}

impl AutoJobConfigDto {
    pub fn new() -> AutoJobConfigDto {
        AutoJobConfigDto {
            data_point: None,
            repo_point: None,
            name: None,
            created_by: None,
            last_ran: None,
            cron_schedule: None,
            backup_path: None,
            next_run: None,
            date_created: None,
            past_jobs: None,
            current_job: None,
            is_running: None,
        }
    }
}

